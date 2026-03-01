use crate::menu::MenuState;
use crate::{despawn_screen, BCollider, BMeshExtra, GameState, Player, PlayerState};
use avian3d::prelude::*;
use bevy::gltf::GltfMeshExtras;
use bevy::prelude::*;
use bevy::scene::SceneInstanceReady;
use bevy::time::common_conditions::on_timer;

pub fn game_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameState::Game), game_setup)
        .add_systems(OnExit(GameState::Game), despawn_screen::<OnGameScreen>)
        .add_systems(
            Update,
            spawn_cube
                .run_if(on_timer(std::time::Duration::from_secs(1)))
                .run_if(in_state(MenuState::Disabled)),
        );
}

// Tag component used to tag entities added on the game screen
#[derive(Component)]
struct OnGameScreen;

fn game_setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    mut player_state: ResMut<NextState<PlayerState>>,
) {
    // Player
    let player_id = commands
        .spawn((
            Name::new("Player"),
            RigidBody::Dynamic,
            Collider::cylinder(0.3, 1.),
            Mesh3d(meshes.add(Cylinder::new(0.3, 1.))),
            MeshMaterial3d(materials.add(Color::srgb_u8(255, 144, 124))),
            Transform::from_xyz(5.0, 0.5, 3.0),
            Player,
            OnGameScreen,
        ))
        .id();
    player_state.set(PlayerState::Id(player_id));
    // light
    commands.spawn((
        Name::new("Light"),
        PointLight {
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0),
        OnGameScreen,
    ));
    commands
        .spawn(SceneRoot(asset_server.load(
            GltfAssetLabel::Scene(1).from_asset("untitled_game.glb"),
        )))
        .observe(on_scene_spawn);
}

fn spawn_cube(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands
        .spawn((
            SceneRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset("untitled_game.glb"))),
            Transform::from_xyz(0., 10., 0.),
        ))
        .observe(on_scene_spawn);
}

fn on_scene_spawn(
    trigger: Trigger<SceneInstanceReady>,
    mut commands: Commands,
    children: Query<&Children>,
    extras: Query<&GltfMeshExtras>,
) {
    for entity in children.iter_descendants(trigger.entity()) {
        if let Ok(gltf_mesh_extra) = extras.get(entity) {
            if let Ok(data) = serde_json::from_str::<BMeshExtra>(&gltf_mesh_extra.value) {
                match data.collider {
                    BCollider::TrimeshFromMesh => {
                        commands.entity(entity).insert((
                            RigidBody::from(data.rigid_body),
                            ColliderConstructor::TrimeshFromMesh,
                            OnGameScreen,
                        ));
                    }
                    BCollider::Cuboid => {
                        // let cube_size = data.cube_size.expect("Cube size must be defined with Cuboid");
                        commands.entity(entity).insert((
                            RigidBody::from(data.rigid_body),
                            // custom properties in blender don't get their y and z swapped automatically like meshes
                            // Collider::cuboid(cube_size.x, cube_size.z, cube_size.y),
                            // I have no idea why adding a cuboid collider seem to scale the mesh rather than creating one with the size specified
                            // but because the lengths are divided by two right away we want to pass in all 2s to get a 1:1 scaling of mesh to collider
                            // ColliderConstructor::Cuboid { x_length: 2., y_length: 2., z_length: 2. },
                            Collider::cuboid(2., 2., 2.),
                            OnGameScreen,
                        ));
                    }
                }
            }
        }
    }
}
