mod crs;

pub use crs::*;
pub mod cameras;
pub mod game;
pub mod menu;

use avian3d::prelude::*;
use bevy::prelude::*;
use bevy::window::CursorGrabMode;

// Generic system that takes a component as a parameter, and will despawn all entities with that component
pub fn despawn_screen<T: Component>(to_despawn: Query<Entity, With<T>>, mut commands: Commands) {
    for entity in &to_despawn {
        commands.entity(entity).despawn_recursive();
    }
}

pub fn not_in_state<S: States>(state: S) -> impl FnMut(Option<Res<State<S>>>) -> bool + Clone {
    move |current_state: Option<Res<State<S>>>| match current_state {
        Some(current_state) => *current_state != state,
        None => false,
    }
}

pub fn pause_physics(mut time: ResMut<Time<Physics>>) {
    time.pause()
}

pub fn unpause_physics(mut time: ResMut<Time<Physics>>) {
    time.unpause()
}

pub fn capture_cursor(mut window: Single<&mut Window>) {
    window.cursor_options.grab_mode = CursorGrabMode::Locked;
    window.cursor_options.visible = false;
}

pub fn release_cursor(mut window: Single<&mut Window>) {
    window.cursor_options.grab_mode = CursorGrabMode::None;
    window.cursor_options.visible = true;
}
