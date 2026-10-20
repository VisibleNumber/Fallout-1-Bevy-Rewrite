import assert from 'node:assert/strict';
import {parseVariables} from './variables.mjs';
const result=parseVariables('// header\nGAME_GLOBAL_VARS:\n// note\nA :=9; ignored\n\nB := -2;\nC\n');
assert.deepEqual(result.values,{0:9,1:-2,2:0});assert.deepEqual(result.definitions.map(v=>v.name),['A','B','C']);
assert.throws(()=>parseVariables('A=0'));assert.throws(()=>parseVariables('GAME_GLOBAL_VARS:\nX=garbage'));assert.throws(()=>parseVariables('GAME_GLOBAL_VARS:\nX=99999999999999'));
console.log('Passed synthetic ordered GAM defaults, comments, implicit zero and malformed input checks.');
