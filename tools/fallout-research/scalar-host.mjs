// Explicit scalar context only. No object pointers, rendering or movement stubs.
export function createScalarHost(initial={}){
 const state=structuredClone(initial),effects=[];
 const integer=v=>{if(!Number.isInteger(v)||v<-2147483648||v>2147483647)throw Error('Host value must be a signed 32-bit integer');return v;};
 const host=(ins,vm)=>{
  const query={op_script_action:'scriptAction',op_cur_map_index:'mapIndex',op_game_time:'gameTime'}[ins.name];
  if(query){if(!(query in state))return false;vm.push(integer(state[query]));return true;}
  const get={op_global_var:'gameGlobals',op_map_var:'mapGlobals',op_local_var:'scriptLocals'}[ins.name];
  if(get){const key=vm.stack.at(-1),vars=state[get];if(!Number.isInteger(key)||!vars||!Object.hasOwn(vars,key))return false;const value=integer(vars[key]);vm.pop();vm.push(value);return true;}
  const set={op_set_global_var:'gameGlobals',op_set_map_var:'mapGlobals',op_set_local_var:'scriptLocals'}[ins.name];
  if(set){const key=vm.stack.at(-2),value=vm.stack.at(-1),vars=state[set];if(!Number.isInteger(key)||!vars||!Object.hasOwn(vars,key))return false;integer(value);vm.pop();vm.pop();const previous=vars[key];vars[key]=value;effects.push({offset:ins.offset,scope:set,index:key,previous,value});return true;}
  if(ins.name==='op_metarule'){
   if(vm.stack.length<2)throw Error('Metarule stack underflow');const rule=vm.stack.at(-2);let value;
   if(rule===14&&Object.hasOwn(state,'mapFlags'))value=+(integer(state.mapFlags)&1?false:true);
   else if(rule===16&&Object.hasOwn(state,'partyCount'))value=integer(state.partyCount);
   else if(rule===22&&Object.hasOwn(state,'isLoadingGame'))value=+!!state.isLoadingGame;
   else return false;
   vm.pop();vm.pop();vm.push(value);return true;
  }
  return false;
 };
 return {state,effects,host};
}
