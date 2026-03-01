use avian3d::prelude::*;
use bevy::core_pipeline::Skybox;
use bevy::prelude::*;
use bevy::render::render_resource::{TextureViewDescriptor, TextureViewDimension};
use untitled_game::cameras::third_person;
use untitled_game::{game, menu, Cubemap, DisplayQuality, GameState, PlayerState, Volume};

fn main() {
    let mut app = App::new();

    app.add_plugins((
        DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: String::from("Untitled_game"),
                // fill the entire browser window
                fit_canvas_to_parent: true,
                // don't hijack keyboard shortcuts like F5, F6, F12, Ctrl+R etc.
                prevent_default_event_handling: false,
                ..default()
            }),
            ..default()
        }),
        PhysicsPlugins::default(),
    ));

    #[cfg(debug_assertions)]
    // show vertices normals
    app.add_plugins(PhysicsDebugPlugin::default());

    // Insert as resource the initial value for the settings resources
    app.insert_resource(DisplayQuality::Medium)
        .insert_resource(Volume(7))
        // Declare the game state, whose starting value is determined by the `Default` trait
        .init_state::<GameState>()
        .init_state::<PlayerState>()
        .add_systems(Startup, setup)
        .add_systems(Update, asset_loaded)
        // Adds the plugins for each state
        .add_plugins((
            menu::menu_plugin,
            game::game_plugin,
            third_person::CameraControllerPlugin,
        ))
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let skybox = asset_server.load("skybox.png");
    commands.insert_resource(Cubemap {
        image_handle: skybox.clone(),
        is_loaded: false,
    });

    // camera
    commands.spawn((
        Name::new("Camera"),
        Camera3d::default(),
        Transform::from_xyz(-2.5, 4.5, 9.0).looking_at(Vec3::ZERO, Vec3::Y),
        third_person::CameraController::default(),
        Skybox {
            image: skybox,
            brightness: 1000.,
            rotation: Default::default(),
        },
    ));
}

fn asset_loaded(
    asset_server: Res<AssetServer>,
    mut images: ResMut<Assets<Image>>,
    mut cubemap: ResMut<Cubemap>,
    mut skybox: Single<&mut Skybox>,
) {
    if !cubemap.is_loaded && asset_server.load_state(&cubemap.image_handle).is_loaded() {
        let image = images.get_mut(&cubemap.image_handle).unwrap();
        // NOTE: PNGs do not have any metadata that could indicate they contain a cubemap texture,
        // so they appear as one texture. The following code reconfigures the texture as necessary.
        if image.texture_descriptor.array_layer_count() == 1 {
            image.reinterpret_stacked_2d_as_array(image.height() / image.width());
            image.texture_view_descriptor = Some(TextureViewDescriptor {
                dimension: Some(TextureViewDimension::Cube),
                ..default()
            });
        }

        skybox.image = cubemap.image_handle.clone();

        cubemap.is_loaded = true;
    }
}
