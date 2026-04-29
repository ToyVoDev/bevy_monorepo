pub mod camera;
pub mod controller;

use bevy::prelude::*;
use avian3d::prelude::*;

#[derive(Component)]
pub struct Player;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugins(PhysicsPlugins::default())
            .add_systems(Startup, (controller::spawn_player, camera::spawn_camera))
            .add_systems(Update, (
                controller::move_player,
                camera::sync_camera,
                cursor_lock,
            ));
    }
}

fn cursor_lock(
    mut windows: Query<&mut Window>,
    mouse: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    let Ok(mut window) = windows.single_mut() else { return };
    if mouse.just_pressed(MouseButton::Left) {
        window.cursor_options.grab_mode = bevy::window::CursorGrabMode::Locked;
        window.cursor_options.visible = false;
    }
    if keys.just_pressed(KeyCode::Escape) {
        window.cursor_options.grab_mode = bevy::window::CursorGrabMode::None;
        window.cursor_options.visible = true;
    }
}
