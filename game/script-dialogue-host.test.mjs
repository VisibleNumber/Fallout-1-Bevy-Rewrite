import assert from 'node:assert/strict';
import { createDialogueHost } from './script-dialogue-host.mjs';

const procedures = Array.from({ length: 22 }, (_, index) => ({ name: `procedure_${index}` }));
procedures[0].name = 'ask_about_road';
procedures[1].name = 'finish_dialogue';
procedures[21].name = 'over_option';
const resolveMessage = (listId, messageId) => ({
  '10:1': 'The road is unsafe.',
  '10:2': 'I can help.',
  '55:114': 'Tell me more about the vault.',
})[`${listId}:${messageId}`];
const call = (binding, name, stack = []) => {
  const vm = { stack: [...stack], status: 'running' };
  const handled = binding.host({ name }, vm);
  return { handled, vm };
};

{
  const binding = createDialogueHost({ intelligence: 5, resolveMessage, procedures });
  const started = call(binding, 'op_start_gdialog', [0, { type: 'pointer', value: 42 }, 3, 7, 11]);
  assert.equal(started.handled, true);
  assert.deepEqual(started.vm.stack, []);
  assert.deepEqual(binding.state, {
    open: true, head: 7, background: 11, reply: null, options: [], messages: [], reaction: 3,
  });

  const reply = call(binding, 'op_gsay_reply', [10, 1]);
  assert.equal(reply.handled, true);
  assert.deepEqual(binding.state.reply, { listId: 10, text: 'The road is unsafe.' });

  const option = call(binding, 'op_gsay_option', [10, 2, 0, 4]);
  assert.equal(option.handled, true);
  assert.deepEqual(binding.state.options[0], {
    listId: 10, text: 'I can help.', procedure: 'ask_about_road', reaction: 4,
  });
}

{
  const binding = createDialogueHost({ intelligence: 4, resolveMessage, procedures });
  const hidden = call(binding, 'op_giq_option', [-3, 10, 2, 0, 1]);
  assert.equal(hidden.handled, true);
  assert.deepEqual(hidden.vm.stack, []);
  assert.deepEqual(binding.state.options, []);
  const shown = call(binding, 'op_giq_option', [4, 10, 2, 0, 1]);
  assert.equal(shown.handled, true);
  assert.equal(binding.state.options.length, 1);
  const originalLike = call(binding, 'op_giq_option', [4, 55, 114, 21, 50]);
  assert.equal(originalLike.handled, true);
  assert.deepEqual(binding.state.options[1], {
    listId: 55, text: 'Tell me more about the vault.', procedure: 'over_option', reaction: 50,
  });
}

{
  const binding = createDialogueHost({ intelligence: 1, resolveMessage, procedures });
  const inline = { type: 'string-offset', value: 77, staticText: 'I will remember that.' };
  const message = call(binding, 'op_gsay_message', [10, inline, 2]);
  assert.equal(message.handled, true);
  assert.deepEqual(binding.state.reply, { listId: 10, text: 'I will remember that.' });
  assert.deepEqual(binding.state.options, [{ listId: -2, text: null, procedure: null, reaction: 50, continue: true }]);
  const end = call(binding, 'op_gsay_end');
  assert.equal(end.handled, true);
  assert.equal(end.vm.status, 'dialogue');
}

{
  const binding = createDialogueHost({ intelligence: 5, resolveMessage, procedures });
  const badMessageStack = [10, 999];
  const badMessage = call(binding, 'op_gsay_reply', badMessageStack);
  assert.equal(badMessage.handled, false);
  assert.deepEqual(badMessage.vm.stack, badMessageStack);
  assert.equal(binding.state.reply, null);

  const badProcedureStack = [10, 2, { type: 'string-offset', value: 12, identifierText: 'not_a_procedure' }, 0];
  const badProcedure = call(binding, 'op_gsay_option', badProcedureStack);
  assert.equal(badProcedure.handled, false);
  assert.deepEqual(badProcedure.vm.stack, badProcedureStack);
  assert.deepEqual(binding.state.options, []);

  assert.equal(call(binding, 'op_unknown', [1]).handled, false);
}

console.log('dialogue host opcode tests passed');
