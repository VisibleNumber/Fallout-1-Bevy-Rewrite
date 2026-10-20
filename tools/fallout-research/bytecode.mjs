// Structural bytecode inspection, not script execution or a high-level decompiler.
import fs from 'node:fs';
import path from 'node:path';

function walk(dir){if(!fs.existsSync(dir))return [];return fs.readdirSync(dir,{withFileTypes:true}).flatMap(e=>e.isSymbolicLink()?[]:e.isDirectory()?walk(path.join(dir,e.name)):[path.join(dir,e.name)]);}

export function loadOpcodeNames(reference){
 const symbols=new Map(),names=new Map();
 for(const file of walk(path.join(reference,'src')).filter(f=>f.endsWith('.h'))){
  for(const m of fs.readFileSync(file,'utf8').matchAll(/\b(OPCODE_\w+)\s*=\s*(0x[\da-f]+)/gi))symbols.set(m[1],Number(m[2]));
 }
 for(const file of walk(path.join(reference,'src')).filter(f=>f.endsWith('.cc'))){
  const source=fs.readFileSync(file,'utf8');
  for(const m of source.matchAll(/interpretAddFunc\(\s*(OPCODE_\w+|0x[\da-f]+)\s*,\s*(\w+)\s*\)/gi)){
   const value=symbols.get(m[1])??Number(m[1]);if(!Number.isFinite(value))continue;
   names.set(value&1023,{name:m[2],referencePath:path.relative(reference,file).replaceAll('\\','/'),referenceLine:source.slice(0,m.index).split('\n').length});
  }
 }
 return names;
}

export function disassemble(buffer,metadata,names=new Map()){
 const instructions=[],issues=[];
 const count=buffer.readUInt32BE(42),identTable=46+count*24;
 const staticBase=identTable+buffer.readUInt32BE(identTable)+8;
 const identifierValue=offset=>{const start=identTable+offset,end=buffer.indexOf(0,start);return offset>=4&&start<staticBase-4&&end>=start&&end<staticBase-4?buffer.toString('latin1',start,end):null;};
 const stringValue=offset=>{
  const start=staticBase+offset,end=buffer.indexOf(0,start);
  return offset>=0&&start>=staticBase&&end>=start&&end<metadata.codeStart?buffer.toString('latin1',start,end):null;
 };
 const regions=[{start:0,end:42,section:'bootstrap'},{start:metadata.codeStart,end:buffer.length,section:'code'}];
 const starts=[...new Set(metadata.procedures.filter(p=>!(p.flags&4)&&p.offset>=metadata.codeStart&&p.offset<buffer.length).map(p=>p.offset))].sort((a,b)=>a-b);
 const labels=new Map(starts.map(offset=>[offset,metadata.procedures.filter(p=>p.offset===offset).map(p=>p.name)]));
 for(const region of regions){let current=[];
  for(let offset=region.start;offset<region.end;){
   if(offset+2>region.end){issues.push({offset,error:'Truncated opcode'});break;}
   const opcode=buffer.readUInt16BE(offset),index=opcode&1023,constant=index===1,size=constant?6:2;
   if(!(opcode&32768)){issues.push({offset,error:'Missing opcode marker',opcode});break;}
   if(offset+size>region.end){issues.push({offset,error:'Truncated immediate',opcode});break;}
   if(labels.has(offset))current=labels.get(offset);
   const resolved=names.get(index),ins={offset,size,opcode:'0x'+opcode.toString(16).padStart(4,'0'),index,name:resolved?.name??(constant?'push':'unknown'),section:region.section,procedures:current};
   if(resolved){ins.referencePath=resolved.referencePath;ins.referenceLine=resolved.referenceLine;}
   else if(!constant)issues.push({offset,error:'Unresolved opcode name',opcode});
   if(constant){ins.raw=buffer.readUInt32BE(offset+2);const type=opcode&0xf7ff;
    if(type===0xc001){ins.valueType='integer';ins.value=buffer.readInt32BE(offset+2);}
    else if(type===0xa001){ins.valueType='float';ins.value=buffer.readFloatBE(offset+2);}
    else if(type===0x9001){ins.valueType='string-offset';ins.staticText=stringValue(ins.raw);ins.identifierText=identifierValue(ins.raw);ins.value=ins.raw;if(ins.staticText===null&&ins.identifierText===null)issues.push({offset,error:'Unresolved string/identifier reference',stringOffset:ins.raw});}
    else{ins.valueType='raw';ins.value=ins.raw;}
   }
   instructions.push(ins);offset+=size;
  }
 }
 const boundaries=new Set(instructions.map(i=>i.offset));
 for(const p of metadata.procedures)if(!(p.flags&4)&&p.offset<buffer.length&&!boundaries.has(p.offset))issues.push({offset:p.offset,error:'Procedure does not start on an instruction',procedure:p.name});
 const calls=[];
 for(let i=1;i<instructions.length;i++){
  const ins=instructions[i],previous=instructions[i-1];
  // Only a directly adjacent integer constant is treated as a resolved target.
  if(previous.offset+previous.size!==ins.offset||previous.valueType!=='integer')continue;
  if(ins.index===5){const p=metadata.procedures[previous.value];if(p)calls.push({offset:ins.offset,from:ins.procedures,targetIndex:previous.value,target:p.name,targetOffset:p.offset,confidence:'adjacent constant'});}
  if(ins.index===4){ins.directJumpTarget=previous.value;if(!boundaries.has(previous.value))issues.push({offset:ins.offset,error:'Direct jump target is not an instruction',target:previous.value});}
 }
 const usage=new Map();for(const i of instructions){if(i.index===1)continue;const key=i.name==='unknown'?i.opcode:i.name;usage.set(key,(usage.get(key)||0)+1);}
 return {confidence:'static structural decode; not observed execution',procedures:metadata.procedures,instructionCount:instructions.length,issues,calls,opcodeUsage:[...usage].map(([name,count])=>({name,count})).sort((a,b)=>b.count-a.count),instructions};
}
