import {createScalarHost} from './scalar-host.mjs';
import {characterStat,characterSkill} from './character.mjs';

// Object/variable research bindings. Structural effects are explicitly opt-in;
// collision, animation, equipment, item merging and scheduling are not simulated.
export function createWorldHost({world,scalarState={},selfId=null,sourceId=null,targetId=null,allowStructuralEffects=false}){
 const scalar=createScalarHost(scalarState),effects=scalar.effects,externals=new Map();
 const named=vm=>{const v=vm.popValue();if(v?.type!=='string-offset'||!v.identifierText)throw Error('Expected resolved identifier');return v.identifierText;};
 const host=(ins,vm)=>{
  if(scalar.host(ins,vm))return true;
  if(ins.index===22){const name=named(vm);if(externals.has(name))throw Error('Duplicate exported variable');externals.set(name,0);return true;}
  if(ins.index===21){const name=named(vm);if(!externals.has(name))throw Error('Unknown exported variable');externals.set(name,vm.popValue());return true;}
  if(ins.index===20){const name=named(vm);if(!externals.has(name))throw Error('Unknown exported variable');vm.pushValue(externals.get(name));return true;}
  const ref={op_dude_obj:world.playerId,op_self_obj:selfId,op_source_obj:sourceId,op_target_obj:targetId,op_obj_being_used_with:targetId};
  if(Object.hasOwn(ref,ins.name)){if(ref[ins.name]!==null)world.get(ref[ins.name]);vm.pushPointer(ref[ins.name]);return true;}
  if(['op_obj_pid','op_tile_num','op_elevation'].includes(ins.name)){const id=vm.popPointer(),o=id===null?null:world.get(id);vm.push(ins.name==='op_elevation'?(o?.elevation??0):ins.name==='op_tile_num'?(o?.tile??-1):(o?.pid??-1));return true;}
  if(ins.name==='op_get_critter_stat'||ins.name==='op_has_skill'){
   const pointer=vm.stack.at(-2);if(pointer?.type!=='pointer')return false;const character=world.get(pointer.value).character;if(!character)return false;
   const index=vm.pop();vm.popPointer();vm.push(ins.name==='op_get_critter_stat'?characterStat(character,index,{gameTime:scalar.state.gameTime}):characterSkill(character,index,{gameDifficulty:scalar.state.gameDifficulty,gameTime:scalar.state.gameTime}));return true;
  }
  if(ins.name==='op_has_trait'){
   const type=vm.stack.at(-3),pointer=vm.stack.at(-2),param=vm.stack.at(-1);if(type!==2||pointer?.type!=='pointer'||!Number.isInteger(param)||param<0||param>=16)return false;
   const player=world.playerId===null?null:world.get(world.playerId);if(!player?.character)return false;if(pointer.value!==0)world.get(pointer.value);vm.pop();vm.popPointer();vm.pop();vm.push(pointer.value===0?0:+player.character.traits.includes(param));return true;
  }
  if(ins.name==='op_obj_is_carrying_obj_pid'||ins.name==='op_obj_carrying_pid_obj'){const pid=vm.pop(),id=vm.popPointer();if(ins.name==='op_obj_is_carrying_obj_pid')vm.push(id===null?0:world.count(id,pid));else vm.pushPointer(id===null?null:world.carrying(id,pid)?.id);return true;}
  if(!allowStructuralEffects)return false;
  if(ins.name==='op_set_light_level'){
   if(!Number.isInteger(scalar.state.nightVisionRank)||scalar.state.nightVisionRank<0)return false;
   const percent=vm.pop(),base=percent===50?40960:percent>50?40960+Math.trunc(percent*24576/100):16384+Math.trunc(percent*24576/100),intensity=Math.max(16384,Math.min(65536,base+scalar.state.nightVisionRank*6553));scalar.state.ambientLight=intensity;effects.push({offset:ins.offset,kind:'ambient-light',percent,intensity});return true;
  }
  if(ins.name==='op_create_object_sid'){
   // Script attachment requires a scheduler; do not silently discard it.
   if(vm.stack.at(-1)!==-1)return false;
   const sid=vm.pop(),elevation=vm.pop(),tile=vm.pop(),pid=vm.pop();if(scalar.state.isLoadingGame){vm.pushPointer(null);return true;}const o=world.create(pid,{tile:tile===-1?0:tile,elevation,sid});vm.pushPointer(o.id);effects.push({offset:ins.offset,kind:'create',id:o.id,pid});return true;
  }
  if(ins.name==='op_add_obj_to_inven'||ins.name==='op_add_mult_objs_to_inven'){const quantity=ins.name==='op_add_mult_objs_to_inven'?vm.pop():1,itemId=vm.popPointer(),ownerId=vm.popPointer();if(itemId===null||ownerId===null)return true;world.add(ownerId,itemId,quantity);effects.push({offset:ins.offset,kind:'inventory-add',ownerId,itemId,quantity});return true;}
  // This changes only logical placement. It deliberately does not claim the
  // renderer/combat/camera side effects of the original override operation.
  if(ins.name==='op_override_map_start'){
   if(world.playerId===null)return false;
   const rotation=vm.pop(),elevation=vm.pop(),y=vm.pop(),x=vm.pop();if(rotation<0||rotation>5)throw Error('Invalid rotation');const o=world.move(world.playerId,200*y+x,elevation);o.rotation=rotation;effects.push({offset:ins.offset,kind:'entry-placement',id:o.id,tile:o.tile,elevation,rotation});return true;
  }
  return false;
 };
 return {world,state:scalar.state,effects,externals,host};
}
