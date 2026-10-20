import assert from 'node:assert/strict';
import {ScriptVM} from './vm.mjs';
import {WorldState} from './world.mjs';
import {createWorldHost} from './world-host.mjs';

const opcode = index => `0x${(0x8000 + index).toString(16).padStart(4, '0')}`;
const ins = (offset, index, name, extra = {}) => ({
  offset, size: index === 1 ? 6 : 2, opcode: opcode(index), index, name, ...extra,
});
const push = (offset, value) => ins(offset, 1, 'push', {valueType: 'integer', value});
const op = (offset, index, name) => ins(offset, index, name);
const pointer = value => ({type: 'pointer', value});
const prototypes = [
  {pid: 100, name: 'container', itemType: 'container'},
  {pid: 101, name: 'item', itemType: 'item'},
];
const makeWorld = () => new WorldState({
  prototypes,
  playerId: 1,
  objects: [
    {id: 1, pid: 100, tile: 10, elevation: 1, inventory: []},
    {id: 2, pid: 101, tile: -1, elevation: 0, inventory: []},
    {id: 3, pid: 101, tile: -1, elevation: 0, inventory: []},
  ],
});
const execute = (instructions, binding, input = {}, options = {}) =>
  new ScriptVM({instructions, host: binding.host, ...options}).run(input);

const world = makeWorld();
const binding = createWorldHost({world, selfId: 1, sourceId: 2, targetId: 3});
const references = execute([
  op(0, 191, 'op_dude_obj'), op(2, 256, 'op_obj_pid'),
  op(4, 188, 'op_self_obj'), op(6, 212, 'op_tile_num'),
  op(8, 189, 'op_source_obj'), op(10, 236, 'op_elevation'),
  op(12, 190, 'op_target_obj'), op(14, 256, 'op_obj_pid'),
  op(16, 192, 'op_obj_being_used_with'), op(18, 212, 'op_tile_num'),
  op(20, 14, 'exit'),
], binding);
assert.equal(references.status, 'halted');
assert.deepEqual(references.stack, [100, 10, 0, 101, -1]);

const missingRef = execute([
  op(0, 188, 'op_self_obj'), op(2, 256, 'op_obj_pid'),
  op(4, 188, 'op_self_obj'), op(6, 212, 'op_tile_num'),
  op(8, 188, 'op_self_obj'), op(10, 236, 'op_elevation'), op(12, 14, 'exit'),
], createWorldHost({world, selfId: null}));
assert.equal(missingRef.status, 'halted');
assert.deepEqual(missingRef.stack, [-1, -1, 0]);
const invalidRef = execute([op(0, 188, 'op_self_obj')], createWorldHost({world, selfId: 999}));
assert.equal(invalidRef.status, 'error');
assert.match(invalidRef.error.message, /Unknown object/);

const createWorld = makeWorld();
const createBinding = createWorldHost({world: createWorld, allowStructuralEffects: true});
const created = execute([
  push(0, 101), push(6, 50), push(12, 1), push(18, -1),
  op(24, 183, 'op_create_object_sid'), op(26, 256, 'op_obj_pid'), op(28, 14, 'exit'),
], createBinding);
assert.equal(created.status, 'halted');
assert.deepEqual(created.stack, [101]);
assert.equal(createWorld.objects.size, 4);
assert.deepEqual(createBinding.effects, [{offset: 24, kind: 'create', id: 4, pid: 101}]);

const defaultWorld = makeWorld();
const defaultBinding = createWorldHost({world: defaultWorld});
const defaultCreate = execute([
  push(0, 101), push(6, 50), push(12, 1), push(18, -1), op(24, 183, 'op_create_object_sid'),
], defaultBinding);
assert.equal(defaultCreate.status, 'blocked');
assert.equal(defaultWorld.objects.size, 3);
assert.deepEqual(defaultBinding.effects, []);
const defaultAdd = execute([
  op(0, 188, 'op_self_obj'), op(2, 189, 'op_source_obj'), op(4, 216, 'op_add_obj_to_inven'),
], defaultBinding);
assert.equal(defaultAdd.status, 'blocked');
assert.equal(defaultWorld.count(1, 101), 0);
assert.deepEqual(defaultBinding.effects, []);

const inventoryWorld = makeWorld();
const inventoryBinding = createWorldHost({
  world: inventoryWorld, selfId: 1, sourceId: 2, targetId: 3, allowStructuralEffects: true,
});
const inventoryChanges = execute([
  op(0, 188, 'op_self_obj'), op(2, 189, 'op_source_obj'), op(4, 216, 'op_add_obj_to_inven'),
  op(6, 188, 'op_self_obj'), op(8, 190, 'op_target_obj'), push(10, 4),
  op(16, 278, 'op_add_mult_objs_to_inven'), op(18, 188, 'op_self_obj'), push(20, 101),
  op(26, 186, 'op_obj_is_carrying_obj_pid'), op(28, 188, 'op_self_obj'), push(30, 101),
  op(36, 269, 'op_obj_carrying_pid_obj'), op(38, 256, 'op_obj_pid'), op(40, 14, 'exit'),
], inventoryBinding);
assert.equal(inventoryChanges.status, 'halted');
assert.deepEqual(inventoryChanges.stack, [5, 101]);
assert.equal(inventoryWorld.get(2).ownerId, 1);
assert.equal(inventoryWorld.get(3).ownerId, 1);
assert.equal(inventoryWorld.count(1, 101), 5);
assert.deepEqual(inventoryBinding.effects, [
  {offset: 4, kind: 'inventory-add', ownerId: 1, itemId: 2, quantity: 1},
  {offset: 16, kind: 'inventory-add', ownerId: 1, itemId: 3, quantity: 4},
]);

// The VM currently accepts pointers as initial tagged values; this host-only
// test opcode supplies the separate resolved identifier tag expected by externals.
const externalWorld = makeWorld();
const externalBinding = createWorldHost({world: externalWorld});
const identifier = {type: 'string-offset', value: 4, identifierText: 'synthetic'};
const externalHost = (instruction, vm) => {
  if (instruction.index === 900) {
    vm.pushValue(identifier);
    return true;
  }
  return externalBinding.host(instruction, vm);
};
const externalVars = new ScriptVM({instructions: [
  op(0, 900, 'inject_identifier'), op(2, 22, 'export_variable'), push(4, 77),
  op(10, 900, 'inject_identifier'), op(12, 21, 'store_external'),
  op(14, 900, 'inject_identifier'), op(16, 20, 'fetch_external'), op(18, 14, 'exit'),
], host: externalHost}).run();
assert.equal(externalVars.status, 'halted');
assert.deepEqual(externalVars.stack, [77]);
assert.equal(externalBinding.externals.get('synthetic'), 77);

const scriptedCreate = execute([
  push(0, 101), push(6, 50), push(12, 1), push(18, 7), op(24, 183, 'op_create_object_sid'),
], createBinding);
assert.equal(scriptedCreate.status, 'blocked');
assert.equal(createWorld.objects.size, 4);
assert.equal(createBinding.effects.length, 1);

console.log('Passed synthetic world-host references, pointer-aware inventory/create effects, external variables and fail-closed writes.');

// These deliberately preserve the reference's special midpoint/discontinuous
// lighting calculation, rather than replacing it with linear interpolation.
for(const [percent,intensity] of [[0,16384],[49,28426],[50,40960],[51,53493],[100,65536]]){
 const lights=createWorldHost({world:makeWorld(),scalarState:{nightVisionRank:0},allowStructuralEffects:true});
 const r=execute([push(0,percent),op(6,233,'op_set_light_level'),op(8,14,'exit')],lights);
 assert.equal(r.status,'halted');assert.equal(lights.state.ambientLight,intensity);
}
const unknownLighting=createWorldHost({world:makeWorld(),allowStructuralEffects:true});
assert.equal(execute([push(0,50),op(6,233,'op_set_light_level')],unknownLighting).status,'blocked');

const stringConstant=ins(0,1,'op_const',{valueType:'string-offset',raw:4,identifierText:'newSynthetic'});
const externals=createWorldHost({world:makeWorld()});
assert.equal(execute([stringConstant,op(6,22,'export_variable'),op(8,14,'exit')],externals).status,'halted');
assert.equal(externals.externals.get('newSynthetic'),0);
