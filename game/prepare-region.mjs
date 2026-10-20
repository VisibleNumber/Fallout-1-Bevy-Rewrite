// Batch geometry preparation. Only authored map IDs/labels are distributable.
import fs from 'node:fs';import path from 'node:path';import {execFileSync} from 'node:child_process';
const root=import.meta.dirname;
const maps=['shadyw','shadye','junkent','junkkill','junkcsno','hubent','hubdwntn','huboldtn','hubheigt','hubwater','laadytum','lablades','lafollwr','lagunrun','raiders','brohdent','glowent'];
if(!process.env.FALLOUT_GAME_PATH)throw Error('Set FALLOUT_GAME_PATH');
const prepared=[],failed=[];
for(const name of maps){
 try{
  const output=execFileSync(process.execPath,[path.join(root,'prepare.mjs')],{env:{...process.env,FALLOUT_MAP:name},encoding:'utf8'});
  const scene=JSON.parse(fs.readFileSync(path.join(root,`assets/scene-${name}.json`)));
  prepared.push({name,map_number:scene.map_number,floors:scene.view3d.floors.length,walls:scene.view3d.props.filter(p=>p.solid).length,doors:scene.view3d.props.filter(p=>p.door).length,roofs:scene.view3d.roofs.length,npcs:scene.actors.length});
  console.log(output.trim());
 }catch(error){failed.push({name,error:error.stderr?.toString().slice(-1200)||error.message});console.log('Skipped '+name);}
}
fs.writeFileSync(path.join(root,'assets/region-catalog.json'),JSON.stringify({schema:1,maps:prepared,failed},null,2));
console.log(JSON.stringify({prepared:prepared.length,failed:failed.map(x=>x.name),walls:prepared.reduce((a,m)=>a+m.walls,0),roofs:prepared.reduce((a,m)=>a+m.roofs,0)}));
