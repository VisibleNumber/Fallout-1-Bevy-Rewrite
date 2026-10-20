import assert from 'node:assert/strict';
import {WorldState} from './world.mjs';

const prototypes = [
  {pid: 100, name: 'container', itemType: 'container'},
  {pid: 101, name: 'stimpack', itemType: 'item'},
  {pid: 102, name: 'key', itemType: 'item'},
];

const loadedInput = [{
  id: 1, pid: 100, tile: 50, elevation: 0, rotation: 2,
  inventory: [{quantity: 2, object: {
    id: 2, pid: 101, tile: -1, elevation: 0,
    inventory: [{quantity: 3, object: {id: 3, pid: 102, tile: -1, elevation: 0}}],
  }}, {object: {id: 4, pid: 102, tile: -1, elevation: 0}}],
}];
const loaded = new WorldState({objects: loadedInput, prototypes, playerId: 1});
assert.equal(loaded.playerId, 1);
assert.equal(loaded.get(1).inventory[0].itemId, 2);
assert.equal(loaded.get(2).ownerId, 1);
assert.equal(loaded.get(3).ownerId, 2);
assert.equal(loaded.get(2).inventory[0].itemId, loaded.get(3).id);
assert.equal(loaded.get(1).inventory[1].quantity, 1, 'missing loaded quantity defaults to one');
assert.equal(loaded.get(4).ownerId, 1);
assert.equal(loaded.count(1, 102), 1, 'count examines direct inventory only');
assert.equal(loaded.count(2, 102), 3);
loadedInput[0].tile = 999;
loadedInput[0].inventory[0].object.tile = 888;
assert.equal(loaded.get(1).tile, 50);
assert.equal(loaded.get(2).tile, -1);

const world = new WorldState({prototypes});
const bag = world.create(100, {tile: 10, elevation: 1});
const defaults = world.create(102);
assert.equal(defaults.tile, -1);
assert.equal(defaults.elevation, 0);
assert.equal(defaults.sid, -1);
assert.equal(world.move(defaults.id, 12, 2), defaults);
assert.equal(defaults.tile, 12);
assert.equal(defaults.elevation, 2);
const first = world.create(101);
const second = world.create(101);
const key = world.create(102);
world.add(bag.id, first.id, 3);
world.add(bag.id, second.id, 2);
world.add(bag.id, key.id, 1);
assert.equal(world.count(bag.id, 101), 5);
assert.equal(world.count(bag.id, 102), 1);
assert.equal(world.get(bag.id).inventory.length, 3, 'same-prototype items remain separate entries');
assert.equal(world.carrying(bag.id, 101).id, first.id);
assert.equal(world.carrying(bag.id, 999), null);

// Partial removal splits a detached quantity; full removal detaches the remainder.
const split = world.remove(bag.id, first.id, 1);
assert.equal(split.quantity, 1);
assert.notEqual(split.object.id, first.id);
assert.equal(split.object.pid, 101);
assert.ok(split.object.ownerId == null);
assert.equal(world.get(split.object.id), split.object);
assert.equal(world.get(bag.id).inventory.find(s => s.itemId === first.id).quantity, 2);
assert.equal(world.count(bag.id, 101), 4);
const detached = world.remove(bag.id, first.id, 2);
assert.equal(detached.object, first);
assert.equal(detached.quantity, 2);
assert.ok(first.ownerId == null);
assert.equal(world.count(bag.id, 101), 2);

const cycleWorld = new WorldState({prototypes});
const outer = cycleWorld.create(100);
const inner = cycleWorld.create(100);
cycleWorld.add(outer.id, inner.id);
assert.throws(() => cycleWorld.add(inner.id, outer.id), /cycle/i);
assert.throws(() => cycleWorld.add(outer.id, inner.id), /owned|owner/i);
assert.throws(() => cycleWorld.move(inner.id, 20, 0), /owned|owner/i);
assert.throws(() => cycleWorld.move(outer.id, -2, 0), /tile/i);
assert.throws(() => cycleWorld.move(outer.id, 20, -1), /elevation/i);
assert.throws(() => cycleWorld.add(outer.id, inner.id, 0), /quantity/i);
assert.throws(() => cycleWorld.remove(outer.id, inner.id, 0), /quantity/i);
assert.throws(() => cycleWorld.remove(outer.id, inner.id, 2), /quantity|available|stack/i);
assert.throws(() => cycleWorld.add(999, inner.id), /object|unknown/i);
assert.throws(() => cycleWorld.add(outer.id, 999), /object|unknown/i);
assert.throws(() => cycleWorld.create(999), /prototype/i);
assert.throws(() => cycleWorld.get(999), /object|unknown/i);

const validOwnership = cycleWorld.snapshot();
const cyclicSnapshot = structuredClone(validOwnership);
cyclicSnapshot.objects.find(o => o.id === outer.id).ownerId = inner.id;
assert.throws(() => WorldState.fromSnapshot(cyclicSnapshot), /cycle/i);
const inconsistentSnapshot = structuredClone(validOwnership);
inconsistentSnapshot.objects.find(o => o.id === inner.id).ownerId = null;
assert.throws(() => WorldState.fromSnapshot(inconsistentSnapshot), /ownership/i);

// Snapshots flatten object references and round-trip without sharing mutable input.
const snapshot = world.snapshot();
const expectedSnapshot = structuredClone(snapshot);
assert.deepEqual(snapshot.schema, 1);
const restored = WorldState.fromSnapshot(snapshot);
assert.deepEqual(restored.snapshot(), expectedSnapshot);
snapshot.objects[0].tile = 9876;
assert.equal(restored.snapshot().objects[0].tile, expectedSnapshot.objects[0].tile);
assert.equal(world.snapshot().objects[0].tile, expectedSnapshot.objects[0].tile);

console.log('Passed synthetic world loading, ownership, inventory quantities, movement rules and snapshot round-trip tests.');
