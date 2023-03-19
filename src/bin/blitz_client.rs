use bevy::prelude::*;
use blitz::{proto::{
    game_service_client::GameServiceClient, session_service_client::SessionServiceClient,
}, bevy::cards::{CardMaterialResource, CardBundle}};
pub fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(Msaa { samples: 4 })
        .insert_resource(TkRuntime {
            rt: tokio::runtime::Runtime::new().unwrap(),
        })
        .insert_resource(BlitzClient::default())
        .add_startup_system(connect_to_session)
        .add_startup_system(add_2d_scene)
        .run();
}

#[derive(Resource)]
pub struct TkRuntime {
    pub rt: tokio::runtime::Runtime,
}
#[derive(Resource, Default)]
pub struct BlitzClient {
    pub session_client: Option<SessionServiceClient<tonic::transport::Channel>>,
    pub game_client: Option<GameServiceClient<tonic::transport::Channel>>,
}

///The grpc client system. This takes a resource to the tokio runtime and the grpc client.
pub fn connect_to_session(mut client: ResMut<BlitzClient>, rt: Res<TkRuntime>) {
    rt.rt.block_on(async move {
        info!("Connecting to session service");
        client.session_client = Some(
            SessionServiceClient::connect("http://localhost:50051")
                .await
                .unwrap(),
        );
    });
}

pub fn add_2d_scene(mut commands: Commands, assets: Res<AssetServer>) {
    commands.spawn(Camera2dBundle::default());
    commands.spawn(SpriteBundle {
        texture: assets.load("card_images/card_image_0000.png"),
        transform: Transform::from_xyz(0.0, 0.0, 0.0),
        ..Default::default()
    });
}

pub fn spawn_cards(mut commands:Commands,cards_mats: Res<CardMaterialResource>){
    let cards=blitz::generate_all_card(2);
    let position=Vec2::new(0.0,0.0);
    for card in cards{
        let mat_res=cards_mats.get_material(&card.into());
        let texture=mat_res.texture.clone();
        commands.spawn(CardBundle::new(card, mat_res.);
       

    }
}
#[derive(Component)]
pub struct Player {}
