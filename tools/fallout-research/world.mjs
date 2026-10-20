// Renderer-independent object state. Inventory transfers are structural primitives,
// not a claim of parity with Fallout's ammo, currency, equipment or stack merging.
const int=(v,label)=>{if(!Number.isSafeInteger(v))throw Error('Invalid '+label);return v;};
const quantity=n=>{if(!Number.isSafeInteger(n)||n<1||n>2147483647)throw Error('Invalid quantity');return n;};
const location=(tile,elevation)=>{if(!Number.isInteger(tile)||tile< -1||tile>=40000||!Number.isInteger(elevation)||elevation<0||elevation>2)throw Error('Invalid tile/elevation');};
export class WorldState{
 constructor({objects=[],prototypes=[],playerId=null}={}){
  this.objects=new Map();this.prototypes=new Map(prototypes.map(p=>[int(p.pid,'prototype'),structuredClone(p)]));this.playerId=playerId;this.nextId=1;
  const load=(raw,ownerId=null)=>{const o=structuredClone(raw);int(o.id,'object id');if(o.id<1||this.objects.has(o.id))throw Error('Duplicate/invalid object id');int(o.pid,'pid');location(o.tile??-1,o.elevation??0);const inv=o.inventory??[];o.tile=o.tile??-1;o.elevation=o.elevation??0;o.rotation=o.rotation??0;o.ownerId=ownerId;o.inventory=[];this.objects.set(o.id,o);this.nextId=Math.max(this.nextId,o.id+1);for(const slot of inv){const child=load(slot.object,o.id);o.inventory.push({itemId:child.id,quantity:quantity(slot.quantity??1)});}return o;};
  for(const o of objects)load(o);if(playerId!==null)this.get(playerId);
 }
 get(id){const o=this.objects.get(id);if(!o)throw Error('Unknown object '+id);return o;}
 create(pid,{tile=-1,elevation=0,sid=-1}={}){const p=this.prototypes.get(pid);if(!p)throw Error('Unknown prototype '+pid);location(tile,elevation);const data={};if((pid>>>24)===0){if(p.itemType===3){data.ammoQuantity=p.ammoCapacity;data.ammoTypePid=p.ammoTypePid;}else if(p.itemType===4)data.quantity=p.quantity;else if(p.itemType===5)data.charges=p.charges;else if(p.itemType===6)data.keyCode=p.keyCode;}const o={id:this.nextId++,pid,fid:p.fid??0,tile,elevation,rotation:0,sid,ownerId:null,inventory:[],data};this.objects.set(o.id,o);return o;}
 move(id,tile,elevation){const o=this.get(id);location(tile,elevation);if(o.ownerId!==null)throw Error('Cannot place an owned object');o.tile=tile;o.elevation=elevation;return o;}
 add(ownerId,itemId,n=1){quantity(n);const owner=this.get(ownerId),item=this.get(itemId);if(item.ownerId!==null)throw Error('Item is already owned');for(let p=owner;p;p=p.ownerId===null?null:this.get(p.ownerId))if(p.id===itemId)throw Error('Ownership cycle');owner.inventory.push({itemId,quantity:n});item.ownerId=ownerId;item.tile=-1;return item;}
 count(ownerId,pid){return this.get(ownerId).inventory.reduce((n,s)=>n+(this.get(s.itemId).pid===pid?s.quantity:0),0);}
 carrying(ownerId,pid){const slot=this.get(ownerId).inventory.find(s=>this.get(s.itemId).pid===pid);return slot?this.get(slot.itemId):null;}
 remove(ownerId,itemId,n=1){quantity(n);const owner=this.get(ownerId),index=owner.inventory.findIndex(s=>s.itemId===itemId);if(index<0)throw Error('Item not carried by owner');const slot=owner.inventory[index],item=this.get(itemId);if(n>slot.quantity)throw Error('Insufficient quantity');if(n===slot.quantity){owner.inventory.splice(index,1);item.ownerId=null;return {object:item,quantity:n};}if(item.inventory.length)throw Error('Cannot split nested inventory');slot.quantity-=n;const split=structuredClone(item);split.id=this.nextId++;split.ownerId=null;this.objects.set(split.id,split);return {object:split,quantity:n};}
 snapshot(){return {schema:1,playerId:this.playerId,nextId:this.nextId,prototypes:structuredClone([...this.prototypes.values()]),objects:structuredClone([...this.objects.values()])};}
 static fromSnapshot(snapshot){
  if(snapshot.schema!==1||!Array.isArray(snapshot.objects))throw Error('Unsupported world snapshot');const w=new WorldState({prototypes:snapshot.prototypes});w.playerId=snapshot.playerId;w.objects.clear();let max=0;
  for(const raw of snapshot.objects){const o=structuredClone(raw);int(o.id,'id');int(o.pid,'pid');if(o.id<1||w.objects.has(o.id))throw Error('Duplicate/invalid object id');location(o.tile,o.elevation);if(!Array.isArray(o.inventory))throw Error('Invalid inventory');w.objects.set(o.id,o);max=Math.max(max,o.id);}
  w.nextId=snapshot.nextId??max+1;if(!Number.isSafeInteger(w.nextId)||w.nextId<=max)throw Error('Invalid next object id');
  const seen=new Set();for(const o of w.objects.values()){for(const slot of o.inventory){quantity(slot.quantity);const child=w.get(slot.itemId);if(seen.has(child.id)||child.ownerId!==o.id)throw Error('Inconsistent inventory ownership');seen.add(child.id);}if(o.ownerId!==null){w.get(o.ownerId);const chain=new Set([o.id]);for(let p=w.get(o.ownerId);p;p=p.ownerId===null?null:w.get(p.ownerId)){if(chain.has(p.id))throw Error('Ownership cycle');chain.add(p.id);}}}
  for(const o of w.objects.values())if(o.ownerId!==null&&!seen.has(o.id))throw Error('Orphaned inventory object');if(w.playerId!==null)w.get(w.playerId);return w;
 }
}
