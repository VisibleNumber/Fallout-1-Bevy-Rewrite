"""Convert local static New Vegas NIF geometry to glTF using PyFFI and Pillow.
Install PyFFI into game/target/conversion-tools; generated assets stay ignored.
Skins/animations are deliberately rejected rather than silently flattened.
"""
import sys, pathlib, time, json, struct, subprocess, math
ROOT=pathlib.Path(__file__).resolve().parent
sys.path.insert(0,str(ROOT/'target/conversion-tools'))
# PyFFI 2.2.3 predates Python's removal of time.clock.
if not hasattr(time,'clock'): time.clock=time.perf_counter
from pyffi.formats.nif import NifFormat
from PIL import Image
name=sys.argv[1].replace('\\','/').lower()
if not name.startswith('meshes/') or '..' in name or not name.endswith('.nif'): raise ValueError('Expected meshes/...nif')
def extract(asset):
    subprocess.run(['node',str(ROOT/'nv-assets.mjs'),'extract',asset],check=True,cwd=ROOT.parent,stdout=subprocess.DEVNULL)
    return ROOT/'assets/nv/raw'/asset
source=extract(name)
data=NifFormat.Data()
with source.open('rb') as stream:data.read(stream)
out=ROOT/'assets/nv/converted'/pathlib.Path(name).stem
out.mkdir(parents=True,exist_ok=True)
blob=bytearray(); views=[];accessors=[];meshes=[];nodes=[];materials=[];images=[];textures=[];bounds=[]
def accessor(values,fmt,kind,component,target):
    while len(blob)%4:blob.append(0)
    offset=len(blob)
    for value in values:blob.extend(struct.pack('<'+fmt,*value))
    views.append({'buffer':0,'byteOffset':offset,'byteLength':len(blob)-offset,'target':target})
    item={'bufferView':len(views)-1,'componentType':component,'count':len(values),'type':kind}
    if kind=='VEC3':item.update(min=[min(v[i] for v in values) for i in range(3)],max=[max(v[i] for v in values) for i in range(3)])
    accessors.append(item);return len(accessors)-1
for root in data.roots:
 for shape in root.tree():
    if not isinstance(shape,NifFormat.NiTriBasedGeom):continue
    if shape.skin_instance:raise ValueError('Skinned meshes require a separate rig conversion')
    geom=shape.data
    if not geom or not geom.vertices:continue
    transform=shape.get_transform()
    # Parent transforms are composed by PyFFI's relative-to traversal.
    if shape is not root:transform=shape.get_transform(root)*root.get_transform()
    vertices=[];normals=[]
    for v in geom.vertices:
        q=v*transform;vertices.append((q.x*0.0142875,q.z*0.0142875,-q.y*0.0142875))
    if geom.has_normals:
        rot=transform.get_matrix_33()
        for n in geom.normals:
            q=n*rot;length=math.sqrt(q.x*q.x+q.y*q.y+q.z*q.z) or 1
            normals.append((q.x/length,q.z/length,-q.y/length))
    tris=list(geom.get_triangles())
    if not tris:continue
    if any(i<0 or i>=len(vertices) for tri in tris for i in tri):raise ValueError('Invalid triangle index')
    attributes={'POSITION':accessor(vertices,'fff','VEC3',5126,34962)}
    if normals:attributes['NORMAL']=accessor(normals,'fff','VEC3',5126,34962)
    if len(geom.uv_sets):attributes['TEXCOORD_0']=accessor([(v.u,1-v.v) for v in geom.uv_sets[0]],'ff','VEC2',5126,34962)
    material={'pbrMetallicRoughness':{'metallicFactor':0,'roughnessFactor':0.85},'doubleSided':True}
    texture_set=None
    for prop in shape.properties:
        if isinstance(prop,NifFormat.BSShaderPPLightingProperty):texture_set=prop.texture_set
    if texture_set and len(texture_set.textures) and texture_set.textures[0]:
        asset=bytes(texture_set.textures[0]).decode('latin1').replace('\\','/').lower()
        png='texture-'+str(len(images))+'.png'
        Image.open(extract(asset)).convert('RGBA').save(out/png)
        images.append({'uri':png});textures.append({'source':len(images)-1})
        material['pbrMetallicRoughness']['baseColorTexture']={'index':len(textures)-1}
    materials.append(material)
    indices=accessor([(i,) for tri in tris for i in tri],'I','SCALAR',5125,34963)
    meshes.append({'name':bytes(shape.name).decode('latin1'),'primitives':[{'attributes':attributes,'indices':indices,'material':len(materials)-1}]})
    nodes.append({'mesh':len(meshes)-1});bounds.extend(vertices)
if not meshes:raise ValueError('No renderable static mesh')
gltf={'asset':{'version':'2.0','generator':'Local PyFFI static NIF adapter'},'scene':0,'scenes':[{'nodes':list(range(len(nodes)))}],'nodes':nodes,'meshes':meshes,'materials':materials,'buffers':[{'uri':'mesh.bin','byteLength':len(blob)}],'bufferViews':views,'accessors':accessors}
if images:gltf.update(images=images,textures=textures)
(out/'mesh.bin').write_bytes(blob)
(out/'model.gltf').write_text(json.dumps(gltf),encoding='utf8')
print(json.dumps({'output':str(out/'model.gltf'),'meshes':len(meshes),'triangles':sum(a['count']//3 for a in accessors if a['type']=='SCALAR'),'textures':len(images),'min':[min(v[i] for v in bounds) for i in range(3)],'max':[max(v[i] for v in bounds) for i in range(3)]}))
