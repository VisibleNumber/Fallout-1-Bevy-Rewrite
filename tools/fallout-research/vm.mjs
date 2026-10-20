// Bounded integer/pointer interpreter subset. Game bindings are supplied separately.
export class ScriptVM {
 constructor({instructions,procedures=[],host=null,maxSteps=10000,maxStack=100000,traceLimit=1000}){
  this.instructions=new Map();for(const ins of instructions){if(this.instructions.has(ins.offset))throw Error('Duplicate instruction offset');this.instructions.set(ins.offset,ins);}
  this.procedures=procedures;this.host=host;this.maxSteps=maxSteps;this.maxStack=maxStack;this.traceLimit=traceLimit;
  if(!Number.isInteger(maxSteps)||maxSteps<1||!Number.isInteger(maxStack)||maxStack<1||!Number.isInteger(traceLimit)||traceLimit<0)throw Error('Invalid execution limits');
 }
 popValue(returning=false){const stack=returning?this.returnStack:this.stack;if(!stack.length)throw Error((returning?'Return':'Data')+' stack underflow');return stack.pop();}
 pop(returning=false){const value=this.popValue(returning);if(!Number.isInteger(value))throw Error('Expected integer, received tagged value');return value;}
 pushValue(value,returning=false){if(typeof value==='number')return this.push(value,returning);if(!['pointer','string-offset'].includes(value?.type)||!Number.isInteger(value.value)||value.value<0||value.value>2147483647)throw Error('Unsupported tagged value');const stack=returning?this.returnStack:this.stack;if(stack.length>=this.maxStack)throw Error('Stack limit');stack.push(Object.freeze({...value}));}
 pushPointer(id){this.pushValue({type:'pointer',value:id??0});}
 popPointer(){const value=this.popValue();if(value===0)return null;if(value?.type!=='pointer')throw Error('Expected object pointer');return value.value||null;}
 truth(value){return typeof value==='number'?value!==0:value.value!==0;}
 push(value,returning=false){if(!Number.isInteger(value)||value<-2147483648||value>2147483647)throw Error('Expected signed 32-bit integer');const stack=returning?this.returnStack:this.stack;if(stack.length>=this.maxStack)throw Error('Stack limit');stack.push(value);}
 jump(target){if(target===-1){this.status='halted';this.pc=target;return;}if(!this.instructions.has(target))throw Error('Invalid instruction target '+target);this.pc=target;}
 slot(index){if(!Number.isInteger(index)||index<0||index>=this.stack.length)throw Error('Stack slot out of bounds');return index;}
 procedure(index){if(!Number.isInteger(index)||index<0||index>=this.procedures.length)throw Error('Invalid procedure index');return this.procedures[index];}
 block(ins,message){this.status='blocked';this.error={offset:ins.offset,opcode:ins.index,message};this.pc=ins.offset;}
 run({entry=0,stack=[],returnStack=[],framePointer=-1,basePointer=-1}={}){
  this.stack=[];this.returnStack=[];this.framePointer=framePointer;this.basePointer=basePointer;this.pc=entry;this.steps=0;this.status='running';this.trace=[];this.error=null;this.critical=false;
  let ins;
  try{
   for(const v of stack)this.pushValue(v);for(const v of returnStack)this.pushValue(v,true);
   while(this.status==='running'&&this.steps<this.maxSteps){
    ins=this.instructions.get(this.pc);if(!ins)throw Error('No instruction at '+this.pc);
    if(this.trace.length<this.traceLimit)this.trace.push({offset:ins.offset,opcode:ins.index,name:ins.name,stackDepth:this.stack.length,returnDepth:this.returnStack.length});
    this.steps++;this.pc=ins.offset+ins.size;
    const op=ins.index;
    if(op===0)continue;
    if(op===1){if(ins.valueType==='integer'){this.push(ins.value);continue;}if(ins.valueType==='string-offset'){this.pushValue({type:'string-offset',value:ins.raw,identifierText:ins.identifierText,staticText:ins.staticText});continue;}this.block(ins,'Unsupported constant type');continue;}
    if(op===2||op===74){this.critical=true;continue;}if(op===3||op===75){this.critical=false;continue;}
    if(op===4){this.jump(this.pop());continue;}
    if(op===5){const p=this.procedure(this.pop());if(p.flags&4){this.block(ins,'Imported procedure requires external program support');continue;}this.jump(p.offset);if(p.flags&16)this.critical=true;continue;}
    if(op===12){this.pushValue(this.popValue(true));continue;}if(op===13){this.pushValue(this.popValue(),true);continue;}
    if(op===14||op===16){this.status='halted';continue;}
    if(op===17){this.status='stopped';continue;}
    if(op===18||op===50){const addr=this.pop(),base=op===18?this.basePointer:this.framePointer;this.pushValue(this.stack[this.slot(base+addr)]);continue;}
    if(op===19||op===49){const addr=this.pop(),value=this.popValue(),base=op===19?this.basePointer:this.framePointer;this.stack[this.slot(base+addr)]=value;continue;}
    if(op===24||op===25){const ret=op===25,a=this.popValue(ret),b=this.popValue(ret);this.pushValue(a,ret);this.pushValue(b,ret);continue;}
    if(op===26){this.popValue();continue;}if(op===27){const a=this.popValue();this.pushValue(a);this.pushValue(a);continue;}
    if(op===28){this.jump(this.pop(true));continue;}
    // POP_EXIT yields to the scheduler rather than halting the whole program.
    if(op===29){this.jump(this.pop(true));if(this.status==='running')this.status='yielded';continue;}
    if(op===41){this.framePointer=this.pop(true);continue;}
    if(op===42){if(this.framePointer<0||this.framePointer>this.stack.length)throw Error('Invalid frame pointer');this.stack.length=this.framePointer;continue;}
    if(op===43){const argc=this.pop();if(argc<0||argc>this.stack.length)throw Error('Invalid argument count');this.push(this.framePointer,true);this.framePointer=this.stack.length-argc;continue;}
    if(op===44){this.basePointer=this.stack.length;continue;}
    if(op===45){this.push(this.procedure(this.pop()).offset);continue;}
    if(op===46){const count=this.pop();if(count<0||count>this.stack.length)throw Error('Invalid dump count');this.stack.length-=count;continue;}
    if(op===47){const condition=this.truth(this.popValue()),target=this.pop();if(!condition)this.jump(target);continue;}
    if(op===48){const condition=this.truth(this.popValue());if(!condition)this.jump(this.pop());continue;}
    if(op===51||op===52){const b=this.popValue(),a=this.popValue();if(a?.type==='string-offset'||b?.type==='string-offset')throw Error('String comparison is not implemented');if((typeof a==='object'&&typeof b==='number'&&b!==0)||(typeof b==='object'&&typeof a==='number'&&a!==0))throw Error('Pointer comparison with nonzero integer');const same=(typeof a==='object'?a.value:a)===(typeof b==='object'?b.value:b);this.push(op===51?+same:+!same);continue;}
    if(op===62||op===63){const b=this.truth(this.popValue()),a=this.truth(this.popValue());this.push(+(op===62?a&&b:a||b));continue;}
    if(op>=51&&op<=66){const b=this.pop(),a=this.pop();let value;
     switch(op){case 51:value=+(a===b);break;case 52:value=+(a!==b);break;case 53:value=+(a<=b);break;case 54:value=+(a>=b);break;case 55:value=+(a<b);break;case 56:value=+(a>b);break;case 57:value=(a+b)|0;break;case 58:value=(a-b)|0;break;case 59:value=Math.imul(a,b);break;case 60:if(!b)throw Error('Division by zero');if(a===-2147483648&&b===-1)throw Error('Signed division overflow');value=Math.trunc(a/b);break;case 61:if(!b)throw Error('Modulo by zero');value=(a%b)|0;break;case 62:value=+(!!a&&!!b);break;case 63:value=+(!!a||!!b);break;case 64:value=a&b;break;case 65:value=a|b;break;case 66:value=a^b;break;case 67:throw Error('Internal unary dispatch');}
     this.push(value);continue;
    }
    if(op===67){this.push(~this.pop());continue;}
    if(op===68){const v=this.pop();this.push(v);continue;} // FLOOR leaves integers unchanged.
    if(op===69){this.push(+!this.truth(this.popValue()));continue;}
    if(op===70){this.push((-this.pop())|0);continue;}
    if(this.host){const before={stack:[...this.stack],returnStack:[...this.returnStack],framePointer:this.framePointer,basePointer:this.basePointer};if(this.host(ins,this)===true)continue;Object.assign(this,before);}
    this.block(ins,'Unsupported opcode; no effects were simulated');
   }
   if(this.status==='running')this.status='limit';
  }catch(e){this.status='error';this.error={offset:ins?.offset??this.pc,opcode:ins?.index??null,message:e.message};}
  return {status:this.status,pc:this.pc,steps:this.steps,stack:[...this.stack],returnStack:[...this.returnStack],framePointer:this.framePointer,basePointer:this.basePointer,error:this.error,trace:this.trace,traceTruncated:this.steps>this.trace.length};
 }
}
