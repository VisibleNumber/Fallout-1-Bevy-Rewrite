import assert from 'node:assert/strict';
import {ScriptVM} from './vm.mjs';
import {createScalarHost} from './scalar-host.mjs';
const push=(offset,value)=>({offset,size:6,index:1,valueType:'integer',value});
const code=[push(0,14),push(6,0),{offset:12,size:2,index:267,name:'op_metarule'},{offset:14,size:2,index:14}];
for(const [mapFlags,expected] of [[0,1],[1,0],[2,1],[3,0]]){const host=createScalarHost({mapFlags});const r=new ScriptVM({instructions:code,host:host.host}).run();assert.equal(r.status,'halted');assert.deepEqual(r.stack,[expected]);}
const missing=new ScriptVM({instructions:code,host:createScalarHost({}).host}).run();assert.equal(missing.status,'blocked');assert.deepEqual(missing.stack,[14,0]);
const initial={gameGlobals:{3:8}},h=createScalarHost(initial);
const writes=[push(0,3),push(6,42),{offset:12,size:2,index:222,name:'op_set_global_var'},push(14,3),{offset:20,size:2,index:221,name:'op_global_var'},{offset:22,size:2,index:14}];
const r=new ScriptVM({instructions:writes,host:h.host}).run();assert.equal(r.status,'halted');assert.deepEqual(r.stack,[42]);assert.equal(h.state.gameGlobals[3],42);assert.equal(initial.gameGlobals[3],8);assert.equal(h.effects.length,1);
const absent=createScalarHost({gameGlobals:{}}),blocked=new ScriptVM({instructions:writes,host:absent.host}).run();assert.equal(blocked.status,'blocked');assert.deepEqual(blocked.stack,[3,42]);assert.deepEqual(absent.effects,[]);
console.log('Passed explicit scalar contexts, first-run bitmask, scoped variable writes, input isolation and missing-state blocking.');
