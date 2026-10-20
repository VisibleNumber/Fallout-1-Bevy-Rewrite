const DIALOGUE_OPS = new Set([
  'op_start_gdialog', 'op_end_dialogue', 'op_gsay_start', 'op_gsay_end',
  'op_gsay_reply', 'op_gsay_option', 'op_gsay_message', 'op_giq_option',
]);

export function createDialogueHost({ intelligence, resolveMessage, procedures } = {}) {
  const state = {
    open: false,
    head: -1,
    background: -1,
    reply: null,
    options: [],
    messages: [],
    reaction: 0,
  };
  const procedureNames = new Set((procedures ?? []).map((procedure) => procedure?.name).filter((name) => typeof name === 'string'));

  const integer = (value) => Number.isInteger(value) && value >= -2147483648 && value <= 2147483647;
  const procedureName = (value) => {
    if (integer(value)) {
      const procedure = procedures?.[value];
      if (typeof procedure?.name === 'string' && procedure.name.length > 0) return procedure.name;
      return null;
    }
    if (value?.type !== 'string-offset') return null;
    for (const name of [value.identifierText, value.staticText]) {
      if (typeof name === 'string' && name.length > 0 && procedureNames.has(name)) return name;
    }
    return null;
  };
  const messageText = (listId, value) => {
    if (integer(value)) {
      if (typeof resolveMessage !== 'function') return null;
      try {
        const text = resolveMessage(listId, value);
        return typeof text === 'string' ? text : null;
      } catch {
        return null;
      }
    }
    if (value?.type === 'string-offset' && typeof value.staticText === 'string') return value.staticText;
    return null;
  };
  const pointerValue = (value) => value?.type === 'pointer' && integer(value.value) && value.value > 0 ? value.value : null;

  const host = (instruction, vm) => {
    if (!DIALOGUE_OPS.has(instruction?.name)) return false;
    const stack = vm?.stack;
    if (!Array.isArray(stack)) return false;
    const peek = (count) => stack.length >= count ? stack.slice(-count) : null;
    const consume = (count) => { stack.length -= count; };

    switch (instruction.name) {
      case 'op_start_gdialog': {
        const args = peek(5);
        if (!args || !integer(args[0]) || pointerValue(args[1]) === null || !integer(args[2]) || !integer(args[3]) || !integer(args[4])) return false;
        const [, , reaction, head, background] = args;
        consume(5);
        state.open = true;
        state.head = head;
        state.background = background;
        state.reaction = reaction;
        state.reply = null;
        state.options = [];
        state.messages = [];
        return true;
      }
      case 'op_end_dialogue':
        state.open = false;
        return true;
      case 'op_gsay_start':
        state.open = true;
        state.messages.push({ kind: 'start' });
        return true;
      case 'op_gsay_end':
        state.open = true;
        state.pause='choice';
        vm.status = 'dialogue';
        return true;
      case 'op_gsay_reply': {
        const args = peek(2);
        if (!args || !integer(args[0])) return false;
        const [listId, rawMessage] = args;
        const text = messageText(listId, rawMessage);
        if (text === null) return false;
        consume(2);
        state.reply = { listId, text };
        state.messages.push({ kind: 'reply', listId, text });
        return true;
      }
      case 'op_gsay_option':
      case 'op_giq_option': {
        const giq = instruction.name === 'op_giq_option';
        const args = peek(giq ? 5 : 4);
        if (!args) return false;
        const iq = giq ? args[0] : null;
        const listId = args[giq ? 1 : 0];
        const rawMessage = args[giq ? 2 : 1];
        const rawProcedure = args[giq ? 3 : 2];
        const reaction = args[giq ? 4 : 3];
        if (!integer(listId) || !integer(reaction) || (giq && !integer(iq))) return false;
        if (giq) {
          if (!integer(intelligence)) return false;
          const visible = iq < 0 ? intelligence <= -iq : intelligence >= iq;
          if (!visible) {
            consume(5);
            return true;
          }
        }
        const text = messageText(listId, rawMessage);
        const procedure = procedureName(rawProcedure);
        if (text === null || procedure === null) return false;
        consume(giq ? 5 : 4);
        state.options.push({ listId, text, procedure, reaction });
        state.reaction = reaction;
        return true;
      }
      case 'op_gsay_message': {
        const args = peek(3);
        if (!args || !integer(args[0]) || !integer(args[2])) return false;
        const [listId, rawMessage, reaction] = args;
        const text = messageText(listId, rawMessage);
        if (text === null) return false;
        consume(3);
        state.reply = { listId, text };
        state.messages.push({ kind: 'message', listId, text });
        state.options.push({ listId: -2, text: null, procedure: null, reaction: 50, continue: true });
        state.pause='message';vm.status='dialogue';
        return true;
      }
      default:
        return false;
    }
  };

  return { host, state };
}
