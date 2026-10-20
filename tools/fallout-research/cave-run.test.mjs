// Local integration only. Game data is read from the user's installation.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {execFileSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {WorldState} from './world.mjs';
if(!process.argv.includes('--integration')){
 console.log('Cave integration skipped; use --integration with a local game and reference source.');
}else{
 const cli=fileURLToPath(new URL('./research.mjs',import.meta.url)),results=[];
 for(const preset of ['combat','diplomat','stealth']){
  const output=JSON.parse(execFileSync(process.execPath,[cli,'cave-run','maps/v13ent.map','premade/'+preset+'.gcd'],{encoding:'utf8',maxBuffer:2*1024*1024}));
  assert.equal(output.status,'halted',JSON.stringify(output));assert.equal(output.error,null);assert.equal(output.blockedInstruction,null);
  const report=JSON.parse(fs.readFileSync(new URL('./reports/sessions/cave-run.json',import.meta.url),'utf8'));
  assert(report.trace.some(i=>i.offset===0));assert(report.effects.some(e=>e.kind==='entry-placement'));assert(report.effects.some(e=>e.kind==='ambient-light'));assert(report.effects.some(e=>e.kind==='inventory-add'));
  const world=WorldState.fromSnapshot(report.world),player=world.get(world.playerId);assert(player.inventory.length>0);assert.equal(player.character.tags.length,4);assert(player.character.baseStats.slice(0,7).every(v=>v>=1&&v<=10));
  for(const slot of player.inventory){const item=world.get(slot.itemId);assert.equal(item.ownerId,player.id);assert(world.prototypes.has(item.pid));const p=world.prototypes.get(item.pid);if(p.itemType===3){assert.equal(item.data.ammoQuantity,p.ammoCapacity);assert.equal(item.data.ammoTypePid,p.ammoTypePid);}}
  assert.deepEqual(world.snapshot(),report.world);fs.writeFileSync(new URL('./reports/sessions/cave-run-'+preset+'.json',import.meta.url),JSON.stringify(report,null,2));results.push({preset,steps:output.steps,inventoryEntries:player.inventory.length});
 }
 fs.writeFileSync(new URL('./reports/sessions/regression-summary.json',import.meta.url),JSON.stringify(results,null,2));console.log('All three original premade character cave-entry simulations completed: '+JSON.stringify(results));
}
