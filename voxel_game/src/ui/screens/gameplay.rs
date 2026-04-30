//! The screen state for the main gameplay.

use bevy::{input::common_conditions::input_just_pressed, prelude::*};
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};

use crate::{
    Pause,
    inventory,
    player,
    ui::audio::procedural::ChiptuneLoop,
    ui::menus::Menu,
    ui::screens::Screen,
};

const GAMEPLAY_BACKGROUND_COLOR: Color = Color::srgb(0.53, 0.81, 0.92);

pub(super) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(Screen::Gameplay), (
        set_gameplay_bg,
        spawn_gameplay,
        crate::despawn_ui_camera,
        player::camera::spawn_camera,
        player::hud::spawn_highlight,
        player::controller::spawn_player,
        capture_cursor,
    ));

    // Toggle pause on key press.
    app.add_systems(
        Update,
        (
            (pause, spawn_pause_overlay, open_pause_menu).run_if(
                in_state(Screen::Gameplay)
                    .and(in_state(Menu::None))
                    .and(input_just_pressed(KeyCode::KeyP).or(input_just_pressed(KeyCode::Escape))),
            ),
            close_menu.run_if(
                in_state(Screen::Gameplay)
                    .and(not(in_state(Menu::None)))
                    .and(input_just_pressed(KeyCode::KeyP)),
            ),
        ),
    );
    app.add_systems(OnExit(Screen::Gameplay), (close_menu, unpause, despawn_gameplay, crate::spawn_ui_camera));
    app.add_systems(
        OnEnter(Menu::None),
        (unpause, capture_cursor).run_if(in_state(Screen::Gameplay)),
    );
}

fn set_gameplay_bg(mut clear_color: ResMut<ClearColor>) {
    clear_color.0 = GAMEPLAY_BACKGROUND_COLOR;
}

fn spawn_gameplay(
    mut commands: Commands,
    mut chiptune_assets: ResMut<Assets<ChiptuneLoop>>,
) {
    // Spawn gameplay music
    let music_handle = chiptune_assets.add(ChiptuneLoop);
    commands.spawn((
        Name::new("Gameplay Music"),
        AudioPlayer(music_handle),
        PlaybackSettings::LOOP,
        DespawnOnExit(Screen::Gameplay),
    ));

    // Spawn a parent entity for all gameplay content so we can despawn it cleanly.
    commands.spawn((
        Name::new("Gameplay Root"),
        Transform::default(),
        Visibility::default(),
        DespawnOnExit(Screen::Gameplay),
        children![
            // Directional light
            (
                Name::new("Sun"),
                DirectionalLight {
                    illuminance: 10_000.0,
                    shadows_enabled: true,
                    ..default()
                },
                Transform::from_rotation(Quat::from_euler(
                    EulerRot::XYZ, -0.5, 0.3, 0.0,
                )),
            ),
        ],
    ));

    // Spawn HUD and inventory
    player::hud::spawn_hud(commands.reborrow());
    inventory::ui::spawn_hotbar(commands.reborrow());
}

fn despawn_gameplay(
    mut cursor_options_query: Query<&mut CursorOptions, With<PrimaryWindow>>,
) {
    // Release cursor when leaving gameplay
    let Ok(mut cursor_options) = cursor_options_query.single_mut() else { return };
    cursor_options.grab_mode = CursorGrabMode::None;
    cursor_options.visible = true;
}

fn unpause(mut next_pause: ResMut<NextState<Pause>>) {
    next_pause.set(Pause(false));
}

fn pause(mut next_pause: ResMut<NextState<Pause>>) {
    next_pause.set(Pause(true));
}

fn spawn_pause_overlay(mut commands: Commands) {
    commands.spawn((
        Name::new("Pause Overlay"),
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
        GlobalZIndex(1),
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.8)),
        DespawnOnExit(Pause(true)),
    ));
}

fn open_pause_menu(mut next_menu: ResMut<NextState<Menu>>) {
    next_menu.set(Menu::Pause);
}

fn close_menu(mut next_menu: ResMut<NextState<Menu>>) {
    next_menu.set(Menu::None);
}

fn capture_cursor(mut cursor_options_query: Query<&mut CursorOptions, With<PrimaryWindow>>) {
    let Ok(mut cursor_options) = cursor_options_query.single_mut() else { return };
    cursor_options.grab_mode = CursorGrabMode::Locked;
    cursor_options.visible = false;
}
