import assert from 'node:assert/strict';
import {ScriptVM} from './vm.mjs';

const opcode = index => `0x${(0x8000 + index).toString(16).padStart(4, '0')}`;
const ins = (offset, index, name, extra = {}) => ({
  offset,
  size: index === 1 ? 6 : 2,
  opcode: opcode(index),
  index,
  name,
  ...extra,
});
const push = (offset, value, valueType = 'integer') => ins(offset, 1, 'push', {valueType, value});
const op = (offset, index, name) => ins(offset, index, name);
const run = (instructions, options = {}, input = {}) => new ScriptVM({instructions, ...options}).run(input);

// Fallout's binary operators pop the right operand first, then the left operand.
const arithmetic = run([
  push(0, 20), push(6, 6), op(12, 58, 'sub'),
  push(14, -7), push(20, 2), op(26, 60, 'div'),
  op(28, 14, 'exit'),
]);
assert.equal(arithmetic.status, 'halted');
assert.deepEqual(arithmetic.stack, [14, -3]);
assert.equal(arithmetic.steps, 7);

// JMP consumes its target and resumes at that instruction offset.
const branch = run([
  push(0, 14), op(6, 4, 'jump'), push(8, 99),
  push(14, 42), op(20, 14, 'exit'),
]);
assert.equal(branch.status, 'halted');
assert.deepEqual(branch.stack, [42]);

// IF branches to its address when its condition is empty/zero.
const conditional = run([
  push(0, 20), push(6, 0), op(12, 47, 'if'),
  push(14, 111), op(20, 14, 'exit'),
]);
assert.equal(conditional.status, 'halted');
assert.deepEqual(conditional.stack, []);

// CALL resolves an internal procedure index to its declared bytecode offset.
const call = run([
  push(0, 1), op(6, 5, 'call'), push(8, 99),
  push(14, 42), op(20, 14, 'exit'),
], {procedures: [{offset: 8, flags: 0}, {offset: 14, flags: 0}]});
assert.equal(call.status, 'halted');
assert.deepEqual(call.stack, [42]);

const importedCall = run([
  push(0, 0), op(6, 5, 'call'), op(8, 14, 'exit'),
], {procedures: [{offset: 8, flags: 4}]});
assert.equal(importedCall.status, 'blocked');
assert.equal(importedCall.error.offset, 6);

const unknown = run([op(0, 999, 'unknown')]);
assert.equal(unknown.status, 'blocked');
assert.equal(unknown.error.offset, 0);
assert.equal(unknown.error.opcode, 999);

const nonIntegerConstant = run([push(0, 1.5, 'float')]);
assert.equal(nonIntegerConstant.status, 'blocked');
assert.equal(nonIntegerConstant.error.offset, 0);

const returnSentinel = run([op(0, 28, 'pop_return')], {}, {returnStack: [-1]});
assert.equal(returnSentinel.status, 'halted');
assert.deepEqual(returnSentinel.returnStack, []);

// A_TO_D and D_TO_A transfer one integer between the independent stacks.
const transfers = run([
  op(0, 12, 'a_to_d'), op(2, 13, 'd_to_a'), op(4, 14, 'exit'),
], {}, {stack: [7], returnStack: [11]});
assert.equal(transfers.status, 'halted');
assert.deepEqual(transfers.stack, [7]);
assert.deepEqual(transfers.returnStack, [11]);

// Global and local addressing use their respective base pointers.
const globalSlot = run([
  push(0, 99), push(6, 0), op(12, 19, 'store_global'),
  push(14, 0), op(20, 18, 'fetch_global'), op(22, 14, 'exit'),
], {}, {stack: [7], basePointer: 0});
assert.equal(globalSlot.status, 'halted');
assert.deepEqual(globalSlot.stack, [99, 99]);
const localSlot = run([
  push(0, 99), push(6, 0), op(12, 49, 'store'),
  push(14, 0), op(20, 50, 'fetch'), op(22, 14, 'exit'),
], {}, {stack: [7], framePointer: 0});
assert.equal(localSlot.status, 'halted');
assert.deepEqual(localSlot.stack, [99, 99]);

const bitwiseNot = run([push(0, 1), op(6, 67, 'bitwise_not'), op(8, 14, 'exit')]);
assert.equal(bitwiseNot.status, 'halted');
assert.deepEqual(bitwiseNot.stack, [-2]);

const floorInteger = run([push(0, -7), op(6, 68, 'floor'), op(8, 14, 'exit')]);
assert.equal(floorInteger.status, 'halted');
assert.deepEqual(floorInteger.stack, [-7]);
const floorUnderflow = run([op(0, 68, 'floor')]);
assert.equal(floorUnderflow.status, 'error');
assert.equal(floorUnderflow.error.offset, 0);

const multiplyOverflow = run([
  push(0, 1073741824), push(6, 2), op(12, 59, 'mul'), op(14, 14, 'exit'),
]);
assert.equal(multiplyOverflow.status, 'halted');
assert.deepEqual(multiplyOverflow.stack, [-2147483648]);

const divisionByZero = run([
  push(0, 5), push(6, 0), op(12, 60, 'div'),
]);
assert.equal(divisionByZero.status, 'error');
assert.equal(divisionByZero.error.message, 'Division by zero');
const signedDivisionOverflow = run([
  push(0, -2147483648), push(6, -1), op(12, 60, 'div'),
]);
assert.equal(signedDivisionOverflow.status, 'error');
assert.equal(signedDivisionOverflow.error.message, 'Signed division overflow');

const stackCap = run([push(0, 1), push(6, 2)], {maxStack: 1});
assert.equal(stackCap.status, 'error');
assert.equal(stackCap.error.message, 'Stack limit');

const swapped = run([push(0, 1), push(6, 2), op(12, 24, 'swap'), op(14, 14, 'exit')]);
assert.equal(swapped.status, 'halted');
assert.deepEqual(swapped.stack, [2, 1]);

const ifTrue = run([
  push(0, 20), push(6, 1), op(12, 47, 'if'), push(14, 42), op(20, 14, 'exit'),
]);
assert.equal(ifTrue.status, 'halted');
assert.deepEqual(ifTrue.stack, [42]);
const whileTrue = run([
  push(0, 20), push(6, 1), op(12, 48, 'while'), push(14, 42), op(20, 14, 'exit'),
]);
assert.equal(whileTrue.status, 'halted');
assert.deepEqual(whileTrue.stack, [20, 42]);

const invalidBranch = run([push(0, 3), op(6, 4, 'jump')]);
assert.equal(invalidBranch.status, 'error');
assert.equal(invalidBranch.error.offset, 6);

// A backward jump remains bounded by the VM step budget.
const limited = run([push(0, 0), op(6, 4, 'jump')], {maxSteps: 5});
assert.equal(limited.status, 'limit');
assert.equal(limited.steps, 5);

console.log('Passed synthetic VM arithmetic, branching, internal/imported calls, unsupported opcodes and step limits.');
