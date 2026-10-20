// Read-only New Vegas BSA access. Extracted assets must remain under ignored game/assets.
import fs from 'node:fs';import path from 'node:path';import zlib from 'node:zlib';
const root=import.meta.dirname;
const candidates=[process.env.FALLOUT_NV_PATH&&path.join(process.env.FALLOUT_NV_PATH,'Data'),path.resolve(root,'../../common/Fallout New Vegas/Data'),path.resolve(root,'../../downloading/22380/Data')].filter(Boolean);
const folder=candidates.find(p=>fs.existsSync(path.join(p,'Fallout - Meshes.bsa')));
if(!folder)throw Error('New Vegas model archives are not available yet');
const files=new Map();
function readAt(fd,offset,count){if(!Number.isInteger(count)||count<0||count>64*1024*1024)throw Error('Invalid BSA read size');const b=Buffer.alloc(count);if(fs.readSync(fd,b,0,count,offset)!==count)throw Error('Archive is incomplete');return b;}
for(const name of fs.readdirSync(folder).filter(n=>/\.bsa$/i.test(n)).sort()){
 const archive=path.join(folder,name);let fd;
 try{
  fd=fs.openSync(archive,'r');
  const h=readAt(fd,0,36);if(h.toString('ascii',0,4)!=='BSA\0'||h.readUInt32LE(4)!==104)continue;
  const n=h.readUInt32LE(16),total=h.readUInt32LE(20),flags=h.readUInt32LE(12);if(n>100000||total>1000000)throw Error('Invalid archive directory');
  const records=readAt(fd,h.readUInt32LE(8),n*16);let cursor=h.readUInt32LE(8)+n*16;const entries=[];
  for(let i=0;i<n;i++){
   const count=records.readUInt32LE(i*16+8),len=readAt(fd,cursor++,1)[0];
   const dir=readAt(fd,cursor,len).toString('latin1').replace(/\0/g,'');cursor+=len;
   const block=readAt(fd,cursor,count*16);cursor+=count*16;
   for(let j=0;j<count;j++){const size=block.readUInt32LE(j*16+8);entries.push({archive,flags,dir,size:size&0x3fffffff,toggle:Boolean(size&0x40000000),offset:block.readUInt32LE(j*16+12)});}
  }
  if(entries.length!==total)throw Error('Incomplete archive directory');
  const names=readAt(fd,cursor,h.readUInt32LE(28)).toString('latin1').split('\0');
  entries.forEach((entry,i)=>{const name=(entry.dir+'/'+names[i]).replaceAll('\\','/').toLowerCase();if(!names[i]||name.includes('..')||!/[.](nif|dds)$/i.test(name))return;files.set(name,entry);});
 }catch(e){process.stderr.write('Skipping '+name+': '+e.message+'\n');}finally{if(fd!==undefined)fs.closeSync(fd);}
}
export function extract(name){name=name.toLowerCase().replaceAll('\\','/');const entry=files.get(name);if(!entry)throw Error('Missing asset '+name);const fd=fs.openSync(entry.archive,'r');try{
 let bytes=readAt(fd,entry.offset,entry.size);if(entry.flags&0x100)bytes=bytes.subarray(bytes[0]+1);
 if(Boolean(entry.flags&4)!==entry.toggle){const expected=bytes.readUInt32LE(0);bytes=zlib.inflateSync(bytes.subarray(4),{maxOutputLength:64*1024*1024});if(bytes.length!==expected)throw Error('Incomplete compressed payload');}
 return bytes;
}finally{fs.closeSync(fd);}}
const [command='list',filter='architecture']=process.argv.slice(2);
if(command==='list'){
 const found=[...files.keys()].filter(p=>p.includes(filter.toLowerCase()));console.log(JSON.stringify({archives:folder,total:files.size,matches:found.length,files:found.slice(0,100)},null,2));
}else if(command==='extract'){
 const name=filter.toLowerCase().replaceAll('\\','/');if(!/^(meshes|textures)\/[a-z0-9_ .\-/]+\.(nif|dds)$/.test(name)||name.includes('..'))throw Error('Unsupported asset path');
 const bytes=extract(name);if(name.endsWith('.nif')&&!bytes.subarray(0,40).toString('ascii').includes('File Format'))throw Error('NIF payload is not downloaded yet');
 if(name.endsWith('.dds')&&bytes.toString('ascii',0,4)!=='DDS ')throw Error('DDS payload is not downloaded yet');
 const output=path.join(root,'assets/nv/raw',name);fs.mkdirSync(path.dirname(output),{recursive:true});fs.writeFileSync(output,bytes);console.log(JSON.stringify({output,bytes:bytes.length}));
}else throw Error('Use list [filter] or extract <asset path>');
