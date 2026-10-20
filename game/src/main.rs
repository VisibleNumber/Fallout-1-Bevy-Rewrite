use bevy::prelude::*;
use serde::Deserialize;
use bevy::render::view::screenshot::{Screenshot,save_to_disk};
#[derive(Deserialize)]struct Frame{file:String,w:f32,h:f32,dx:f32,dy:f32}
#[derive(Deserialize)]struct Placed{file:String,x:f32,y:f32,z:f32}
#[derive(Deserialize,Resource)]struct SceneData{sprites:Vec<Placed>,player:Vec<Frame>,spawn:[f32;2]}
#[derive(Component)]struct Player;
#[derive(Resource)]struct Facing(usize);
fn main(){let data=std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"),"/assets/scene.json")).expect("Run node game/prepare.mjs first");let scene:SceneData=serde_json::from_str(&data).unwrap();App::new().insert_resource(scene).insert_resource(Facing(2)).insert_resource(ClearColor(Color::BLACK)).add_plugins(DefaultPlugins.set(AssetPlugin{file_path:concat!(env!("CARGO_MANIFEST_DIR"),"/assets").into(),..default()}).set(ImagePlugin::default_nearest()).set(WindowPlugin{primary_window:Some(Window{title:"Fallout Rewrite — original-art renderer".into(),resolution:(1280,960).into(),..default()}),..default()})).add_systems(Startup,setup).add_systems(Update,(control,capture)).run();}
fn capture(mut commands:Commands,time:Res<Time>,mut done:Local<bool>){if !*done&&time.elapsed_secs()>8.{commands.spawn(Screenshot::primary_window()).observe(save_to_disk(concat!(env!("CARGO_MANIFEST_DIR"),"/assets/first-render.png")));*done=true;}}
fn setup(mut commands:Commands,data:Res<SceneData>,assets:Res<AssetServer>){
 commands.spawn((Camera2d,Transform::from_xyz(data.spawn[0],-data.spawn[1],1000.)));
 for s in &data.sprites{commands.spawn((Sprite::from_image(assets.load(s.file.clone())),Transform::from_xyz(s.x,-s.y,s.z)));}
 let f=&data.player[2];commands.spawn((Player,Sprite::from_image(assets.load(f.file.clone())),Transform::from_xyz(data.spawn[0]+f.dx,-data.spawn[1]+f.h/2.-f.dy,20.)));
 commands.spawn((Text::new("FALLOUT REWRITE | Original local artwork\nWASD pan | Arrows move player | R reset\nRendering prototype: combat and interactions not connected"),TextFont{font_size:16.,..default()},TextColor(Color::srgb(0.3,1.,0.3)),Node{position_type:PositionType::Absolute,left:px(12),top:px(12),padding:UiRect::all(px(8)),..default()},BackgroundColor(Color::srgba(0.,0.,0.,0.85))));
}
fn control(keys:Res<ButtonInput<KeyCode>>,time:Res<Time>,data:Res<SceneData>,assets:Res<AssetServer>,mut facing:ResMut<Facing>,mut player:Single<(&mut Transform,&mut Sprite),With<Player>>,mut camera:Single<&mut Transform,(With<Camera2d>,Without<Player>)>){
 let dt=time.delta_secs().min(0.05);let mut d=Vec2::ZERO;if keys.pressed(KeyCode::KeyW){d.y+=1.;}if keys.pressed(KeyCode::KeyS){d.y-=1.;}if keys.pressed(KeyCode::KeyA){d.x-=1.;}if keys.pressed(KeyCode::KeyD){d.x+=1.;}camera.translation+=d.extend(0.)*dt*300.;
 let mut p=Vec2::ZERO;if keys.pressed(KeyCode::ArrowUp){p.y+=1.;facing.0=0;}if keys.pressed(KeyCode::ArrowDown){p.y-=1.;facing.0=3;}if keys.pressed(KeyCode::ArrowLeft){p.x-=1.;facing.0=5;}if keys.pressed(KeyCode::ArrowRight){p.x+=1.;facing.0=2;}
 let(t,s)=&mut *player;t.translation+=p.normalize_or_zero().extend(0.)*dt*100.;s.image=assets.load(data.player[facing.0].file.clone());
 if keys.just_pressed(KeyCode::KeyR){let f=&data.player[2];t.translation=Vec3::new(data.spawn[0]+f.dx,-data.spawn[1]+f.h/2.-f.dy,20.);camera.translation.x=data.spawn[0];camera.translation.y=-data.spawn[1];}
}
