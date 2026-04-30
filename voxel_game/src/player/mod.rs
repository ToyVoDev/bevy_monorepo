pub mod camera;
pub mod controller;
pub mod hud;
pub mod interaction;

use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};
use avian3d::prelude::*;
use crate::PausableSystems;

#[derive(Component)]
pub struct Player;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugins(PhysicsPlugins::default())
            .init_resource::<interaction::TargetedVoxel>()
            .add_systems(Update, (
                controller::move_player,
                camera::sync_camera,
                cursor_lock,
                interaction::update_targeted_voxel,
                hud::update_highlight,
                hud::update_coordinates,
            ).in_set(PausableSystems))
            .add_systems(Update, (
                interaction::handle_break_place,
                interaction::handle_pickup,
            ).in_set(PausableSystems));
    }
}

fn cursor_lock(
    mut cursor_options_query: Query<&mut CursorOptions, With<PrimaryWindow>>,
    mouse: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    let Ok(mut cursor_options) = cursor_options_query.single_mut() else { return };
    if mouse.just_pressed(MouseButton::Left) {
        cursor_options.grab_mode = CursorGrabMode::Locked;
        cursor_options.visible = false;
    }
    if keys.just_pressed(KeyCode::Escape) {
        cursor_options.grab_mode = CursorGrabMode::None;
        cursor_options.visible = true;
    }
}
