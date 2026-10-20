// Local asset adapter for the game, never a distributable asset bundle.
// Floor-to-hex coverage is an initial approximation. Original animation motion
// offsets and frame timing still require runtime comparison.
import {execFileSync} from 'node:child_process';
import {characterStat,characterSkill} from '../tools/fallout-research/character.mjs';
import fs from 'node:fs';import path from 'node:path';import zlib from 'node:zlib';
import {indexDat,decodeDat,parseMap,parsePro} from '../tools/fallout-research/research.mjs';
const root=import.meta.dirname,out=path.join(root,'assets'),game=process.env.FALLOUT_GAME_PATH;if(!game)throw Error('Set FALLOUT_GAME_PATH to your Fallout installation');
const supportedMaps={v13ent:35,vault13:6,shadyw:26,shadye:25};
const mapName=(process.env.FALLOUT_MAP||'v13ent').toLowerCase();
if(!/^[a-z0-9]{1,12}$/.test(mapName))throw Error('Invalid map name');
const all=new Map(),buffers=new Map();for(const source of ['MASTER.DAT','CRITTER.DAT']){const b=fs.readFileSync(path.join(game,source));buffers.set(source,b);for(const e of indexDat(b,source))all.set(e.path,e);}
const read=name=>{name=name.toLowerCase();const loose=path.join(game,'DATA',name);if(fs.existsSync(loose))return fs.readFileSync(loose);const e=all.get(name);if(!e)throw Error('Missing '+name);return decodeDat(buffers.get(e.source),e);};
const palette=read('color.pal'),types=['items','critters','scenery','walls','tiles','misc'],lists=Object.fromEntries(types.map(t=>[t,read(`art/${t}/${t}.lst`).toString('latin1').split(/\r?\n/).map(s=>s.split(/[,; ]/)[0].toLowerCase())]));
const protos=new Map();for(const e of all.values())if(e.path.endsWith('.pro')){const bytes=read(e.path),p=parsePro(bytes);if(p.type===2&&bytes.length>=36)p.sceneryType=bytes.readInt32BE(32);protos.set(p.pid,p);}const map=parseMap(read(`maps/${mapName}.map`),true,protos);
fs.mkdirSync(out,{recursive:true});const crc=b=>{let c=0xffffffff;for(const v of b){c^=v;for(let i=0;i<8;i++)c=(c>>>1)^((c&1)?0xedb88320:0);}return(c^0xffffffff)>>>0;};
function png(w,h,rgba){const chunk=(s,b)=>{const t=Buffer.from(s),n=Buffer.alloc(4),c=Buffer.alloc(4);n.writeUInt32BE(b.length);c.writeUInt32BE(crc(Buffer.concat([t,b])));return Buffer.concat([n,t,b,c]);};const head=Buffer.alloc(13);head.writeUInt32BE(w);head.writeUInt32BE(h,4);head[8]=8;head[9]=6;const scan=Buffer.alloc((w*4+1)*h);for(let y=0;y<h;y++)rgba.copy(scan,y*(w*4+1)+1,y*w*4,(y+1)*w*4);return Buffer.concat([Buffer.from([137,80,78,71,13,10,26,10]),chunk('IHDR',head),chunk('IDAT',zlib.deflateSync(scan)),chunk('IEND',Buffer.alloc(0))]);}
const cache=new Map(),errors=[];
function frame(name,rotation=0,index=0){const key=name+':'+rotation+':'+index;if(cache.has(key))return cache.get(key);const b=read(name),count=b.readUInt16BE(8),start=62+b.readUInt32BE(34+rotation*4);let offset=start;for(let i=0;i<index;i++)offset+=12+b.readUInt32BE(offset+4);const w=b.readUInt16BE(offset),h=b.readUInt16BE(offset+2),n=b.readUInt32BE(offset+4);if(w*h!==n||offset+12+n>b.length)throw Error('Invalid FRM '+name);const rgba=Buffer.alloc(n*4);for(let i=0;i<n;i++){const c=b[offset+12+i];if(c){for(let k=0;k<3;k++)rgba[i*4+k]=Math.min(255,palette[c*3+k]*4);rgba[i*4+3]=255;}}const file=`sprite-${mapName}-${cache.size}.png`;fs.writeFileSync(path.join(out,file),png(w,h,rgba));const f={file,w,h,dx:b.readInt16BE(10+rotation*2)+b.readInt16BE(offset+8),dy:b.readInt16BE(22+rotation*2)+b.readInt16BE(offset+10),count};cache.set(key,f);return f;}
function screen(tile){const c=199-tile%200,r=Math.floor(tile/200);return [24*c+16*r+8*(c%2),-6*c+12*r+6*(c%2)];}
const sprites=[];
for(const layer of map.elevations.filter(e=>e.elevation===0))for(let i=0;i<layer.tiles.length;i++){const id=layer.tiles[i].floor;if([1,70,71].includes(id))continue;try{const f=frame('art/tiles/'+lists.tiles[id]),c=99-i%100,r=Math.floor(i/100);sprites.push({...f,x:48*c+32*r+f.w/2,y:-12*c+24*r+f.h/2,z:0});}catch(e){errors.push(e.message);}}
for(const o of map.objects.filter(o=>o.tile>=0&&o.elevation===0&&!(o.flags&1))){try{const type=(o.fid>>>24)&15;if(type>5||type===1)continue;let name=lists[types[type]][o.fid&4095];if(['block.frm','scrblk.frm','trnblk.frm','reserved.frm'].includes(name))continue;if(type===1)name=name.slice(0,6)+'aa.frm';const f=frame('art/'+types[type]+'/'+name,o.rotation),[x,y]=screen(o.tile);sprites.push({...f,x:x+16+f.dx,y:y+8-f.h/2+f.dy,z:(o.flags&8)?1:10+y/10000+(o.tile%200)/10000000,id:o.id,tile:o.tile});}catch(e){errors.push(e.message);}}
const walking=[];for(let r=0;r<6;r++){const count=frame('art/critters/hmjmpsab.frm',r).count;walking.push(Array.from({length:count},(_,i)=>frame('art/critters/hmjmpsab.frm',r,i)));}
const walkable=Array(40000).fill(false);const floorLayer=map.elevations.find(e=>e.elevation===0);if(!floorLayer)throw Error(`Map ${mapName} has no elevation-zero floors`);const floors=floorLayer.tiles;for(let t=0;t<40000;t++){const col=199-t%200,row=Math.floor(t/200),i=Math.floor(row/2)*100+99-Math.floor(col/2);if(floors[i]&&![1,70,71].includes(floors[i].floor))walkable[t]=true;}
const dirs=[[-1,199,200,201,1,-200],[-201,-1,200,1,-199,-200]];for(const o of map.objects){const type=(o.fid>>>24)&15;if(o.tile<0||o.elevation!==0||(o.flags&17)||![1,2,3].includes(type))continue;walkable[o.tile]=false;if(o.flags&2048)for(const delta of dirs[o.tile%2])if(o.tile+delta>=0&&o.tile+delta<40000)walkable[o.tile+delta]=false;}
const requestedSpawn=mapName==='v13ent'?17690:map.enteringTile;let spawnTile=requestedSpawn;if(!Number.isInteger(spawnTile)||spawnTile<0||spawnTile>=40000||!walkable[spawnTile]){const [sx,sy]=screen(Math.max(0,Math.min(39999,requestedSpawn|0)));let nearest=-1,best=Infinity;for(let t=0;t<40000;t++)if(walkable[t]){const [x,y]=screen(t),d=(x-sx)**2+(y-sy)**2;if(d<best){best=d;nearest=t;}}if(nearest<0)throw Error(`Map ${mapName} has no walkable floor tiles`);spawnTile=nearest;}
const uiList=read('art/intrface/intrface.lst').toString('latin1').split(/\r?\n/);const ui=[];for(const [id,x,y] of [[16,0,0],[47,211,41],[18,210,62],[6,523,7],[13,526,40],[59,526,78],[57,526,59]]){const name=uiList[id].split(/[,; ]/)[0].toLowerCase();ui.push({...frame('art/intrface/'+name),x,y});}const world_map=frame('art/intrface/'+uiList[135].split(/[,; ]/)[0].toLowerCase());

const reference=process.env.FALLOUT_REFERENCE_PATH||path.resolve(root,'../../fallout-research/reference/fallout1-ce');
if(!fs.existsSync(path.join(reference,'src')))throw Error('Set FALLOUT_REFERENCE_PATH to the local reference checkout for startup script execution');
const env={...process.env,FALLOUT_REFERENCE_PATH:reference};const cli=path.join(root,'../tools/fallout-research/research.mjs'),reportRoot=path.join(root,'../tools/fallout-research/reports');
if(!fs.existsSync(path.join(reportRoot,'prototypes.json')))execFileSync(process.execPath,[cli,'scan'],{env,stdio:'pipe'});
execFileSync(process.execPath,[cli,'cave-run','maps/v13ent.map','premade/combat.gcd'],{env,stdio:'pipe'});
const startup=JSON.parse(fs.readFileSync(path.join(reportRoot,'sessions/cave-run.json')));if(startup.status!=='halted')throw Error('Starting inventory script did not complete');
const pc=startup.world.objects.find(o=>o.id===startup.world.playerId),inventoryNames=read('text/english/game/pro_item.msg').toString('latin1'),messages=new Map([...inventoryNames.matchAll(/\{(\d+)\}\s*\{[^}]*\}\s*\{([^}]*)\}/g)].map(m=>[Number(m[1]),m[2]])),invenList=read('art/inven/inven.lst').toString('latin1').split(/\r?\n/);const inventory=[];
function itemRecord(item,quantity){const proto=protos.get(item.pid),fileName=invenList[proto.inventoryFid&4095].split(/[,; ]/)[0].toLowerCase(),icon=frame('art/inven/'+fileName);return {id:item.id,pid:item.pid,name:messages.get(proto.messageId)||('Item '+item.pid),description:messages.get(proto.messageId+1)||'',file:icon.file,quantity,weight:proto.weight,item_type:proto.itemType,ammo_quantity:item.data.ammoQuantity??item.data.quantity??0,ammo_capacity:proto.ammoCapacity??0,ammo_pid:item.data.ammoTypePid??proto.ammoTypePid??0,caliber:proto.caliber??0,ammo_pack_size:proto.itemType===4?proto.quantity:0,heal_min:proto.stats?.[0]===-2&&proto.stats[1]===35?proto.amounts[0]:0,heal_max:proto.stats?.[0]===-2&&proto.stats[1]===35?proto.amounts[1]:0};}
for(const slot of pc.inventory){const item=startup.world.objects.find(o=>o.id===slot.itemId),record=itemRecord(item,slot.quantity);const existing=inventory.find(i=>i.pid===item.pid&&i.ammo_quantity===record.ammo_quantity);if(existing)existing.quantity+=slot.quantity;else inventory.push(record);}
const loot_containers=[];for(const obj of startup.world.objects.filter(o=>o.id!==pc.id&&o.tile>=0&&o.elevation===0&&o.inventory?.length)){const contents=[];for(const slot of obj.inventory){const item=startup.world.objects.find(o=>o.id===slot.itemId),record=itemRecord(item,0);let dest=inventory.find(i=>i.pid===record.pid&&i.ammo_quantity===record.ammo_quantity);if(!dest){record.ammo_quantity=0;inventory.push(record);dest=record;}contents.push({item_id:dest.id,quantity:slot.quantity});}loot_containers.push({id:obj.id,tile:obj.tile,name:messages.get(protos.get(obj.pid).messageId)||'Container',contents});}
const ap_light=frame('art/intrface/'+uiList[83].split(/[,; ]/)[0].toLowerCase());
const inventory_background=frame('art/intrface/'+uiList[48].split(/[,; ]/)[0].toLowerCase());


const weapon_art=[];for(const item of inventory.filter(i=>i.item_type===3)){const code=protos.get(item.pid).animationCode;if(!code)continue;const letter=String.fromCharCode(100+code-1),standing=[],walk=[],attacking=[];for(let r=0;r<6;r++){standing.push(frame('art/critters/hmjmps'+letter+'a.frm',r));const count=frame('art/critters/hmjmps'+letter+'b.frm',r).count;walk.push(Array.from({length:count},(_,i)=>frame('art/critters/hmjmps'+letter+'b.frm',r,i)));const mode=protos.get(item.pid).extendedFlags&15,attackSuffix=mode===6?'j':mode===3?'g':mode===4?'f':null;if(attackSuffix){const name='art/critters/hmjmps'+letter+attackSuffix+'.frm',count=frame(name,r).count;attacking.push(Array.from({length:count},(_,i)=>frame(name,r,i)));}else attacking.push([]);}weapon_art.push({pid:item.pid,standing,walking:walk,attacking});}

const critterText=read('text/english/game/pro_crit.msg').toString('latin1'),critterMessages=new Map([...critterText.matchAll(/\{(\d+)\}\s*\{[^}]*\}\s*\{([^}]*)\}/g)].map(m=>[Number(m[1]),m[2]]));
// Critters stay separate from static scenery so health and corpse art can change.
const actors=[];for(const o of map.objects.filter(o=>o.tile>=0&&o.elevation===0&&!(o.flags&1)&&((o.fid>>>24)&15)===1)){const proto=protos.get(o.pid),base=lists.critters[o.fid&4095].slice(0,6),standing=[],dead=[];let visible=true;for(let r=0;r<6;r++){let idle;for(const idleName of ['art/critters/'+base+'aa.frm']){try{idle=frame(idleName,r);break;}catch{}}if(!idle){visible=false;break;}standing.push(idle);let corpse=idle;for(const [deathName,deathRotation] of [['art/critters/'+base+'ba.fr'+r,0],['art/critters/'+base+'ba.frm',r]]){try{const death=frame(deathName,deathRotation);corpse=frame(deathName,deathRotation,death.count-1);break;}catch{}}dead.push(corpse);}const stat=i=>proto.baseStats[i]+proto.bonusStats[i];actors.push({visible,id:o.id,tile:o.tile,name:critterMessages.get(proto.messageId)||('Critter '+o.pid),hostile:mapName==='v13ent',hp:o.data.hp,max_hp:stat(7),ap:stat(8),armor_class:stat(9),thresholds:Array.from({length:7},(_,i)=>stat(17+i)),resistances:Array.from({length:7},(_,i)=>stat(24+i)),melee_damage:stat(11),skill:65+Math.trunc((stat(0)+stat(5))/2)+proto.skills[3],rotation:o.rotation,standing,dead});}
const weapons=inventory.filter(i=>i.item_type===3).map(i=>{const p=protos.get(i.pid),mode=p.extendedFlags&15,ranged=mode>=6,skillIndex=mode<=2?3:(mode<=4?4:(mode===5?5:0));return {damage_type:p.damageType,pid:i.pid,min_damage:p.minDamage,max_damage:p.maxDamage+(ranged?0:characterStat(pc.character,11)),range:p.maxRange1,ap_cost:p.actionPointCost1,skill:characterSkill(pc.character,skillIndex),ranged,perception:characterStat(pc.character,1)};});
const objectTexts=new Map();for(const [type,name] of [[0,'pro_item'],[1,'pro_crit'],[2,'pro_scen'],[3,'pro_wall'],[5,'pro_misc']]){const text=read('text/english/game/'+name+'.msg').toString('latin1');objectTexts.set(type,new Map([...text.matchAll(/\{(\d+)\}\s*\{[^}]*\}\s*\{([^}]*)\}/g)].map(m=>[Number(m[1]),m[2]])));}
const examinables=map.objects.filter(o=>o.tile>=0&&o.elevation===0&&!(o.flags&1)).flatMap(o=>{const p=protos.get(o.pid),messages=objectTexts.get(o.pid>>>24),name=p&&messages?.get(p.messageId);return name?[{id:o.id,tile:o.tile,name,description:messages.get(p.messageId+1)||''}]:[];});
const shot_blocked=Array(40000).fill(false);for(const o of map.objects){const type=(o.fid>>>24)&15;if(o.tile>=0&&o.elevation===0&&!(o.flags&1)&&!(o.flags&0x80000000)&&[2,3].includes(type))shot_blocked[o.tile]=true;}
const player_combat={id:pc.id,tile:spawnTile,hp:characterStat(pc.character,7),max_hp:characterStat(pc.character,7),ap:characterStat(pc.character,8),max_ap:characterStat(pc.character,8),armor_class:characterStat(pc.character,9)};
const unarmed={damage_type:0,pid:-1,min_damage:1,max_damage:characterStat(pc.character,11)+2,range:1,ap_cost:3,skill:characterSkill(pc.character,3),ranged:false,perception:characterStat(pc.character,1)};
const unarmed_attacking=[];for(let r=0;r<6;r++){const name="art/critters/hmjmpsaq.frm",count=frame(name,r).count;unarmed_attacking.push(Array.from({length:count},(_,i)=>frame(name,r,i)));}
const player=[];for(let r=0;r<6;r++)player.push(frame('art/critters/hmjmpsaa.frm',r));const [x,y]=screen(spawnTile);
const briefing=JSON.parse(fs.readFileSync(path.join(root,'content/overseer-briefing.json'),'utf8').replace(/^\uFEFF/,''));
const overText=read('text/english/dialog/over.msg').toString('latin1'),overMessages=new Map([...overText.matchAll(/\{(\d+)\}\s*\{[^}]*\}\s*\{([^}]*)\}/g)].map(m=>[Number(m[1]),m[2]]));
const pages=[];for(const id of briefing.messages){const text=overMessages.get(id);if(!text)throw Error('Missing briefing message '+id);let page='';for(const word of text.split(/\s+/)){if(page.length+word.length>420){pages.push(page);page='';}page+=(page?' ':'')+word;}if(page)pages.push(page);}
const dialogue_definition={id:briefing.id,speaker:briefing.speaker,start:'page-0',flags:briefing.flags,nodes:Object.fromEntries(pages.map((text,i)=>['page-'+i,{text,choices:[{id:i===pages.length-1?'enter_cave':'continue',label:i===pages.length-1?'Enter the cave':'Continue',next:i===pages.length-1?null:'page-'+(i+1),effects:i===pages.length-1?briefing.finish_effects:[]}]}]))};
const dialogue_portrait=frame('art/heads/ovrsrnf1.frm');
const exit_grids=map.objects.filter(o=>o.tile>=0&&o.elevation===0&&Number.isInteger(o.data?.map)&&Number.isInteger(o.data?.tile)).map(o=>({tile:o.tile,map:o.data.map,destination_tile:o.data.tile,elevation:o.data.elevation??0,rotation:o.data.rotation??0}));
// Geometry carries original bitmap references only; generated content remains local.
const groundPoint=(x,y)=>[(12*x-16*y)/384,(6*x+24*y)/384];
// Find an opaque original-art rectangle for solid wall surfaces. No pixels are repainted.
const wallPatches=new Map();
function wallPatch(name,rotation){
 const key=name+':'+rotation;if(wallPatches.has(key))return wallPatches.get(key);
 const b=read(name),at=62+b.readUInt32BE(34+rotation*4),w=b.readUInt16BE(at),h=b.readUInt16BE(at+2),pixels=b.subarray(at+12,at+12+w*h);
 const heights=Array(w).fill(0);let best={area:0,x:0,y:0,w:1,h:1};
 for(let y=0;y<h;y++){
  for(let x=0;x<w;x++)heights[x]=pixels[y*w+x]!==0?heights[x]+1:0;
  const stack=[];
  for(let x=0;x<=w;x++){
   const current=x<w?heights[x]:0;let start=x;
   while(stack.length&&stack.at(-1).height>current){const col=stack.pop(),width=x-col.start,area=width*col.height;start=col.start;if(area>best.area)best={area,x:col.start,y:y-col.height+1,w:width,h:col.height};}
   if(!stack.length||stack.at(-1).height<current)stack.push({start,height:current});
  }
 }
 if(best.area<16)throw Error('No opaque wall texture region: '+name);
 const inset=0.5,uv=[(best.x+inset)/w,(best.y+inset)/h,(best.x+best.w-inset)/w,(best.y+best.h-inset)/h];wallPatches.set(key,uv);return uv;
}
const view3d={floors:[],props:[],roofs:[]};
for(let i=0;i<floorLayer.tiles.length;i++){
 const id=floorLayer.tiles[i].floor;if([1,70,71].includes(id))continue;
 const f=frame('art/tiles/'+lists.tiles[id]),c=99-i%100,r=Math.floor(i/100);
 view3d.floors.push({file:f.file,x:2*c-0.5,z:2*r+0.75});
}
for(let i=0;i<floorLayer.tiles.length;i++){
 const id=floorLayer.tiles[i].roof;if([1,70,71].includes(id))continue;
 const f=frame('art/tiles/'+lists.tiles[id]),c=99-i%100,r=Math.floor(i/100);
 view3d.roofs.push({file:f.file,x:2*c-0.5,z:2*r+0.75});
}
for(const o of map.objects.filter(o=>o.tile>=0&&o.elevation===0&&!(o.flags&1))){
 const kind=(o.fid>>>24)&15;if(![0,2,3].includes(kind))continue;
 const name=lists[types[kind]][o.fid&4095];
 if(!name||['block.frm','scrblk.frm','trnblk.frm','reserved.frm'].includes(name))continue;
 const f=frame('art/'+types[kind]+'/'+name,o.rotation),[sx,sy]=screen(o.tile),[x,z]=groundPoint(sx+16,sy+8);
 view3d.props.push({source_pid:o.pid,source_id:o.id,material_slot:kind===3?'wall':protos.get(o.pid)?.sceneryType===0?'door':'prop',file:f.file,w:f.w,h:f.h,x,z,kind,tile:o.tile,door:kind===2&&protos.get(o.pid)?.sceneryType===0,solid:kind===3&&!(o.flags&8)&&f.h>25,uv:process.env.FALLOUT_TEXTURE_WALLS==='1'&&kind===3&&!(o.flags&8)&&f.h>25?wallPatch('art/'+types[kind]+'/'+name,o.rotation):[0,0,1,1]});
}
const scene={view3d,map_name:mapName,map_number:map.mapNumber,exit_grids,world_map,dialogue_definition,dialogue_portrait,examinables,ap_light,carry_weight:characterStat(pc.character,12),loot_catalog:loot_containers,loot_containers:mapName==='v13ent'?loot_containers:[],unarmed_attacking,shot_blocked,actors,weapons,unarmed,player_combat,sprites,ui,inventory,inventory_background,weapon_art,player_name:pc.character.name,hit_points:characterStat(pc.character,7,{gameTime:264600}),armor_class:characterStat(pc.character,9,{gameTime:264600}),player,walking,walkable,spawn_tile:spawnTile,spawn:[x+16,y+8],errors,limitations:['Original sorting/light/roof behavior not yet reproduced.','Player movement and cave combat are prototypes; full original behavior is incomplete.']};
fs.writeFileSync(path.join(out,`scene-${mapName}.json`),JSON.stringify(scene));if(mapName==='v13ent')fs.writeFileSync(path.join(out,'scene.json'),JSON.stringify(scene));console.log(JSON.stringify({map:mapName,spawnTile,exits:exit_grids.length,sprites:sprites.length,images:cache.size,errors:errors.length,examples:errors.slice(0,6)}));
