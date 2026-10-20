import assert from 'node:assert/strict';
import {ScriptVM} from './vm.mjs';

const opcode = index => `0x${(0x8000 + index).toString(16).padStart(4, '0')}`;
const ins = (offset, index, name, extra = {}) => ({
  offset, size: index === 1 ? 6 : 2, opcode: opcode(index), index, name, ...extra,
});
const push = (offset, value) => ins(offset, 1, 'push', {valueType: 'integer', value});
const op = (offset, index, name) => ins(offset, index, name);
const run = (instructions, input = {}, options = {}) => new ScriptVM({instructions, ...options}).run(input);
const pointer = value => ({type: 'pointer', value});

const apiVM = new ScriptVM({instructions: [op(0, 14, 'exit')]});
apiVM.stack = [];
apiVM.returnStack = [];
apiVM.pushPointer(17);
assert.deepEqual(apiVM.stack[0], pointer(17));
assert.equal(apiVM.popPointer(), 17);
apiVM.pushPointer(0);
assert.equal(apiVM.popPointer(), null);
apiVM.stack.push(0);
assert.equal(apiVM.popPointer(), null, 'integer zero is also a null pointer');
apiVM.stack.push(8);
assert.throws(() => apiVM.popPointer(), /pointer/i);

// Generic slot fetch/store and DUP preserve the tagged representation.
const storage = run([
  op(0, 27, 'dup'), push(2, 0), op(8, 49, 'store'),
  push(10, 0), op(16, 50, 'fetch'), op(18, 27, 'dup'), op(20, 14, 'exit'),
], {stack: [pointer(23)], framePointer: 0});
assert.equal(storage.status, 'halted');
assert.deepEqual(storage.stack, [pointer(23), pointer(23), pointer(23)]);

// A_TO_D and D_TO_A transfer tagged values between data and return stacks.
const transfer = run([
  op(0, 13, 'd_to_a'), op(2, 12, 'a_to_d'), op(4, 14, 'exit'),
], {stack: [pointer(7)]});
assert.equal(transfer.status, 'halted');
assert.deepEqual(transfer.stack, [pointer(7)]);
assert.deepEqual(transfer.returnStack, []);

for (const stack of [[pointer(0), 0], [0, pointer(0)]]) {
  const equalNull = run([op(0, 51, 'equal'), op(2, 14, 'exit')], {stack});
  assert.equal(equalNull.status, 'halted');
  assert.deepEqual(equalNull.stack, [1]);
}
const pointerEquality = run([
  op(0, 51, 'equal'), op(2, 14, 'exit'),
], {stack: [pointer(23), pointer(23)]});
assert.equal(pointerEquality.status, 'halted');
assert.deepEqual(pointerEquality.stack, [1]);

for (const index of [51, 52]) {
  const invalidComparison = run([op(0, index, index === 51 ? 'equal' : 'not_equal')], {
    stack: [pointer(23), 1],
  });
  assert.equal(invalidComparison.status, 'error');
  assert.match(invalidComparison.error.message, /Pointer comparison with nonzero integer/);
}
const invalidArithmetic = run([op(0, 57, 'add')], {stack: [pointer(23), 1]});
assert.equal(invalidArithmetic.status, 'error');
assert.match(invalidArithmetic.error.message, /tagged value/);

const truthyPointer = run([
  op(0, 47, 'if'), push(2, 1), op(8, 14, 'exit'),
  push(14, 42), op(20, 14, 'exit'),
], {stack: [14, pointer(23)]});
assert.equal(truthyPointer.status, 'halted');
assert.deepEqual(truthyPointer.stack, [1]);
const nullPointer = run([
  op(0, 47, 'if'), push(2, 1), op(8, 14, 'exit'),
  push(14, 42), op(20, 14, 'exit'),
], {stack: [14, pointer(0)]});
assert.equal(nullPointer.status, 'halted');
assert.deepEqual(nullPointer.stack, [42]);

console.log('Passed synthetic tagged-pointer transfers, storage, comparison guards, arithmetic rejection and IF truth tests.');
