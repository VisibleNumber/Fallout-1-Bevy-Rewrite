// Original INT dialogue execution, independent of Bevy. Unsupported host calls
// stop the transaction; they are never replaced with guessed quest outcomes.
import fs from 'node:fs';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import {indexDat,decodeDat,parseMap,parsePro,parseInt,safeName} from '../tools/fallout-research/research.mjs';
import {parseCharacter,characterStat} from '../tools/fallout-research/character.mjs';
import {parseVariables} from '../tools/fallout-research/variables.mjs';
import {loadOpcodeNames,disassemble} from '../tools/fallout-research/bytecode.mjs';
import {WorldState} from '../tools/fallout-research/world.mjs';
import {createWorldHost} from '../tools/fallout-research/world-host.mjs';
import {ScriptVM} from '../tools/fallout-research/vm.mjs';
import {createDialogueHost} from './script-dialogue-host.mjs';

const root=import.meta.dirname;
export class ScriptGame {
 constructor(game=process.env.FALLOUT_GAME_PATH||path.resolve(root,'../../common/Fallout')) {
  this.game=game;this.entries=new Map();this.buffers=new Map();
  for(const name of ['MASTER.DAT','CRITTER.DAT']) {const b=fs.readFileSync(path.join(game,name));this.buffers.set(name,b);for(const entry of indexDat(b,name))this.entries.set(entry.path,entry);}
  this.scriptList=this.read('scripts/scripts.lst').toString('latin1').split(/\r?\n/);
  this.prototypes=[...this.entries.values()].filter(e=>e.path.endsWith('.pro')).map(e=>parsePro(this.read(e.path)));
  this.names=loadOpcodeNames(process.env.FALLOUT_REFERENCE_PATH||path.resolve(root,'../../fallout-research/reference/fallout1-ce'));
  if(!this.names.size)throw Error('A local reference checkout is required to resolve script opcodes');
 }
 read(name) {name=safeName(name);const loose=path.join(this.game,'DATA',name);if(fs.existsSync(loose))return fs.readFileSync(loose);const entry=this.entries.get(name);if(!entry)throw Error('Missing installed asset '+name);return decodeDat(this.buffers.get(entry.source),entry);}
 message(list,id) {
  const line=this.scriptList[list-1];if(!line)throw Error('Unknown dialogue message list '+list);
  const script=line.split('.')[0].trim().toLowerCase();
  const text=this.read(`text/english/dialog/${script}.msg`).toString('latin1');
  const messages=new Map([...text.matchAll(/\{(\d+)\}\s*\{[^}]*\}\s*\{([^}]*)\}/g)].map(m=>[Number(m[1]),m[2].replace(/\s+/g,' ').trim()]));
  if(!messages.has(id))throw Error(`Missing message ${script}:${id}`);return messages.get(id);
 }
 openOverseer({inventory=[],characterPath='premade/combat.gcd',hp=43}={}) {
  if(!['premade/combat.gcd','premade/diplomat.gcd','premade/stealth.gcd'].includes(characterPath))throw Error('Unsupported player character');
  const index=this.scriptList.findIndex(s=>/^over\.int\b/i.test(s));
  const localCount=Number(this.scriptList[index]?.match(/local_vars\s*=\s*(\d+)/i)?.[1]);
  if(index<0||!Number.isInteger(localCount))throw Error('Missing OVER script declaration');
  const map=parseMap(this.read('maps/vault13.map'),false,new Map(this.prototypes.map(p=>[p.pid,p])));
  const attachment=map.mapScripts.find(s=>(s.scriptIndex&0xffffff)===index);
  if(!attachment)throw Error('Overseer script is not attached to Vault 13');
  const character=parseCharacter(this.read(characterPath));
  const pcPid=0x1000000,world=new WorldState({objects:map.objects,prototypes:[...this.prototypes.filter(p=>p.pid!==pcPid),{pid:pcPid,type:1,character}]});
  const player=world.create(pcPid,{tile:map.enteringTile,elevation:map.enteringElevation});player.character=character;player.data.hp=hp;world.playerId=player.id;this.syncInventory(world,inventory);
  const globals=parseVariables(this.read('data/vault13.gam').toString('latin1')).values;
  const mapGlobals=this.entries.has('maps/vault13.gam')?parseVariables(this.read('maps/vault13.gam').toString('latin1'),'MAP_GLOBAL_VARS:').values:{};
  const sharedBytes=this.read('scripts/vault13.int'),sharedMeta=parseInt(sharedBytes),sharedCode=disassemble(sharedBytes,sharedMeta,this.names);
  const sharedContext=createWorldHost({world,scalarState:{scriptAction:0,gameGlobals:globals,mapGlobals,scriptLocals:{}},allowStructuralEffects:true});
  const sharedVm=new ScriptVM({instructions:sharedCode.instructions,procedures:sharedMeta.procedures,host:sharedContext.host,maxSteps:100000,traceLimit:0});
  const sharedResult=sharedVm.run();if(sharedResult.status!=='halted')throw Error('Vault 13 initialization blocked: '+JSON.stringify({...sharedResult,trace:undefined}));
  return this.execute({script:'over',perks:{},externals:[...sharedContext.externals],selfId:attachment.ownerId,world:world.snapshot(),scalar:{scriptAction:11,mapIndex:6,mapFlags:map.flags,
   gameTime:264600,gameDifficulty:'normal',nightVisionRank:0,isLoadingGame:false,gameGlobals:globals,mapGlobals,
   scriptLocals:Object.fromEntries(Array.from({length:localCount},(_,i)=>[i,0]))}});
 }
 execute(session,procedure=null,continuing=false,rootCall=false) {
  if(session.script!=='over')throw Error('Only the original Overseer conversation is currently bound');
  const bytes=this.read('scripts/over.int'),meta=parseInt(bytes),decoded=disassemble(bytes,meta,this.names);
  if(decoded.issues.length)throw Error('Invalid script bytecode');
  const world=WorldState.fromSnapshot(session.world);
  const context=createWorldHost({world,scalarState:session.scalar,selfId:session.selfId,sourceId:world.playerId,allowStructuralEffects:true});
  for(const [name,value] of session.externals??[])context.externals.set(name,value);
  const dialogue=createDialogueHost({intelligence:characterStat(world.get(world.playerId).character,4),resolveMessage:(list,id)=>this.message(list,id),procedures:meta.procedures});
  const presentation=[];
  const host=(ins,vm)=>{
   if(dialogue.host(ins,vm))return true;
   if(ins.name==='op_has_trait'&&vm.stack.at(-3)===0) {
    const param=vm.pop(),id=vm.popPointer();vm.pop();
    if(!Number.isInteger(param)||param<0||param>=119)throw Error('Invalid perk index');
    vm.push(id===null?0:(session.perks?.[param]??0));return true;
   }
   if(ins.name==='op_script_overrides'){presentation.push({kind:'script-overrides'});return true;}
   return context.host(ins,vm);
  };
  const vm=new ScriptVM({instructions:decoded.instructions,procedures:meta.procedures,host,maxSteps:100000,traceLimit:40});
  let initial=continuing?{entry:session.vm.pc,stack:session.vm.stack,returnStack:session.vm.returnStack,basePointer:session.vm.basePointer,framePointer:session.vm.framePointer}:{};
  if(procedure!==null) {
   if(!session.vm)throw Error('No initialized program snapshot');
   const proc=meta.procedures.find(p=>p.name===procedure);if(!proc||proc.argc!==0)throw Error('Invalid dialogue callback');
   // A callback receives a synthetic zero-argument entry frame while retaining
   // initialized program variables. The suspended conversation continues later.
   initial={entry:proc.offset,stack:[...session.vm.stack,0],returnStack:[-1],basePointer:session.vm.basePointer,framePointer:-1};
  }
  const result=vm.run(initial);
  const snapshot={pc:result.pc,stack:result.stack,returnStack:result.returnStack,basePointer:result.basePointer,framePointer:result.framePointer};
  let owner=rootCall?null:(session.rootVm??(procedure!==null?session.vm:null));
  if(owner)owner={...owner,stack:result.stack.slice(0,owner.stack.length)};
  const next={...session,world:world.snapshot(),scalar:context.state,externals:[...context.externals],vm:snapshot,rootVm:owner,step:(session.step??0)+1};
  let status=result.status;
  if(result.status==='halted'&&dialogue.state.reply&&dialogue.state.options.length){
   status='dialogue';dialogue.state.pause='choice';dialogue.state.open=true;
   if(owner){next.vm=owner;next.rootVm=owner;}
  }else if(result.status==='halted'&&owner){
   return this.execute({...next,vm:owner,rootVm:null},null,true);
  }
  if(status==='dialogue'&&!next.rootVm&&dialogue.state.pause==='choice')next.rootVm=snapshot;
  next.view=dialogue.state;next.phase=status==='halted'?'closed':(dialogue.state.pause??null);
  return {status,error:result.error,blockedInstruction:decoded.instructions.find(i=>i.offset===result.error?.offset)?.name,
   dialogue:dialogue.state,presentation,effects:context.effects,session:next,trace:result.trace};
 }
 syncInventory(world,inventory) {
  if(!Array.isArray(inventory)||inventory.length>10000)throw Error('Invalid inventory bridge');
  const player=world.get(world.playerId);
  for(const slot of player.inventory)world.get(slot.itemId).ownerId=null;
  player.inventory=[];
  for(const record of inventory){
   if(!Number.isInteger(record.quantity)||record.quantity<0||!Number.isInteger(record.pid))throw Error('Invalid inventory item');
   if(!record.quantity)continue;
   const item=world.create(record.pid);item.bridgeId=record.id;
   if(record.item_type===3){item.data.ammoQuantity=record.ammo_quantity;item.data.ammoTypePid=record.ammo_pid;}
   else if(record.item_type===4)item.data.quantity=record.ammo_quantity;
   world.add(player.id,item.id,record.quantity);
  }
 }
 reopen(session,inventory,hp){
  if(session.phase!=='closed')throw Error('Conversation is already active');
  const world=WorldState.fromSnapshot(session.world);this.syncInventory(world,inventory);world.get(world.playerId).data.hp=hp;
  return this.execute({...session,world:world.snapshot(),scalar:{...session.scalar,scriptAction:11},rootVm:null},'talk_p_proc',false,true);
 }
 choose(session,option,expectedStep) {
  if(expectedStep!==session.step)throw Error('Stale dialogue choice');
  if(!Number.isInteger(option)||option<0||option>=(session.view?.options.length??0))throw Error('Invalid dialogue choice');
  const choice=session.view.options[option];
  if(choice.continue)return this.execute(session,null,true);
  if(!choice.procedure)throw Error('Dialogue choice has no callback');
  return this.execute(session,choice.procedure);

 }
}
if(process.argv[1]&&path.resolve(process.argv[1])===fileURLToPath(import.meta.url)) {
 try {const request=JSON.parse(fs.readFileSync(0,'utf8')||'{"action":"start"}'),game=new ScriptGame();
  const result=request.action==='start'?game.openOverseer(request):request.action==='reopen'?game.reopen(request.session,request.inventory,request.hp):game.choose(request.session,request.option,request.expected_step);
  process.stdout.write(JSON.stringify(result));
 }catch(error){process.stdout.write(JSON.stringify({status:'error',error:{message:error.message}}));process.exitCode=1;}
}
