import assert from 'node:assert/strict';
import fs from 'node:fs';
import {disassemble} from './bytecode.mjs';

// Entirely synthetic program: noop bootstrap, integer constant, call, exit.
const b=Buffer.alloc(68);for(let i=0;i<42;i+=2)b.writeUInt16BE(0x8000,i);
b.writeUInt32BE(0,42);b.writeUInt32BE(0,46);b.writeUInt32BE(0xffffffff,50);b.writeUInt32BE(0xffffffff,54);
b.writeUInt16BE(0xc001,58);b.writeInt32BE(0,60);b.writeUInt16BE(0x8005,64);b.writeUInt16BE(0x800e,66);
const metadata={codeStart:58,procedures:[{name:'synthetic',offset:58,flags:0}]},names=new Map([[0,{name:'noop'}],[1,{name:'push'}],[5,{name:'call'}],[14,{name:'exit'}]]);
const result=disassemble(b,metadata,names);
assert.equal(result.issues.length,0);assert.equal(result.instructionCount,24);
assert.equal(result.calls.length,1);assert.equal(result.calls[0].target,'synthetic');
assert.equal(result.instructions.at(-3).size,6);assert.equal(result.instructions.at(-3).value,0);
const bad=Buffer.from(b);bad.writeUInt16BE(0xc001,66);
assert(disassemble(bad,metadata,names).issues.some(i=>i.error==='Truncated immediate'));
const jump=Buffer.from(b);jump.writeInt32BE(59,60);jump.writeUInt16BE(0x8004,64);
assert(disassemble(jump,metadata,names).issues.some(i=>i.error==='Direct jump target is not an instruction'));
const unknown=disassemble(b,metadata,new Map());assert(unknown.issues.some(i=>i.error==='Unresolved opcode name'));
console.log('Passed synthetic instruction widths, call resolution, truncation, unknown opcodes and jump-boundary checks.');
if(process.argv.includes('--integration')){
 const read=p=>JSON.parse(fs.readFileSync(new URL('./reports/'+p,import.meta.url)));
 const audit=read('bytecode-summary.json');assert.equal(audit.scripts,952);assert.equal(audit.scriptsWithIssues,0);assert.equal(audit.instructions,757163);
 const cave=read('bytecode/scripts/v13cave.int.json');assert(cave.calls.some(c=>c.from.includes('map_enter_p_proc')&&c.target==='base_inventory'));
 const links=read('links/maps/v13ent.map.json');assert.equal(links.mapScript,'scripts/v13cave.int');assert(links.dependencies.every(d=>d.exists));assert.equal(links.scriptLinks.length,24);assert.equal(links.objects.length,1496);
 console.log('Passed complete local script audit and opening-cave dependency checks.');
}
