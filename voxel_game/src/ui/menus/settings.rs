//! The settings menu.
//!
//! Additional settings and accessibility options should go here.

use bevy::{audio::Volume, input::common_conditions::input_just_pressed, prelude::*};

use crate::ui::{menus::Menu, screens::Screen, theme::prelude::*};
use crate::Settings;

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<Settings>();
    app.add_systems(OnEnter(Menu::Settings), spawn_settings_menu);
    app.add_systems(
        Update,
        go_back.run_if(in_state(Menu::Settings).and(input_just_pressed(KeyCode::Escape))),
    );

    app.add_systems(
        Update,
        (
            update_global_volume_label,
            update_sensitivity_label,
            update_fov_label,
            update_render_distance_label,
            update_coords_label,
        )
            .run_if(in_state(Menu::Settings)),
    );
}

fn spawn_settings_menu(mut commands: Commands) {
    commands.spawn((
        widget::ui_root("Settings Menu"),
        GlobalZIndex(2),
        DespawnOnExit(Menu::Settings),
        children![
            widget::header("Settings"),
            settings_grid(),
            widget::button("Back", go_back_on_click),
        ],
    ));
}

fn settings_grid() -> impl Bundle {
    (
        Name::new("Settings Grid"),
        Node {
            display: Display::Grid,
            row_gap: Val::Px(10.0),
            column_gap: Val::Px(30.0),
            grid_template_columns: RepeatedGridTrack::px(2, 400.0),
            ..default()
        },
        children![
            (
                widget::label("Master Volume"),
                Node { justify_self: JustifySelf::End, ..default() }
            ),
            volume_widget(),
            (
                widget::label("Mouse Sensitivity"),
                Node { justify_self: JustifySelf::End, ..default() }
            ),
            sensitivity_widget(),
            (
                widget::label("FOV"),
                Node { justify_self: JustifySelf::End, ..default() }
            ),
            fov_widget(),
            (
                widget::label("Render Distance"),
                Node { justify_self: JustifySelf::End, ..default() }
            ),
            render_distance_widget(),
            (
                widget::label("Show Coordinates"),
                Node { justify_self: JustifySelf::End, ..default() }
            ),
            coords_widget(),
        ],
    )
}

fn volume_widget() -> impl Bundle {
    (
        Name::new("Volume Widget"),
        Node { justify_self: JustifySelf::Start, ..default() },
        children![
            widget::button_small("-", lower_global_volume),
            (
                Name::new("Current Volume"),
                Node { padding: UiRect::horizontal(Val::Px(10.0)), justify_content: JustifyContent::Center, ..default() },
                children![(widget::label(""), GlobalVolumeLabel)],
            ),
            widget::button_small("+", raise_global_volume),
        ],
    )
}

fn sensitivity_widget() -> impl Bundle {
    (
        Name::new("Sensitivity Widget"),
        Node { justify_self: JustifySelf::Start, ..default() },
        children![
            widget::button_small("-", lower_sensitivity),
            (
                Name::new("Current Sensitivity"),
                Node { padding: UiRect::horizontal(Val::Px(10.0)), justify_content: JustifyContent::Center, ..default() },
                children![(widget::label(""), SensitivityLabel)],
            ),
            widget::button_small("+", raise_sensitivity),
        ],
    )
}

fn fov_widget() -> impl Bundle {
    (
        Name::new("FOV Widget"),
        Node { justify_self: JustifySelf::Start, ..default() },
        children![
            widget::button_small("-", lower_fov),
            (
                Name::new("Current FOV"),
                Node { padding: UiRect::horizontal(Val::Px(10.0)), justify_content: JustifyContent::Center, ..default() },
                children![(widget::label(""), FovLabel)],
            ),
            widget::button_small("+", raise_fov),
        ],
    )
}

fn render_distance_widget() -> impl Bundle {
    (
        Name::new("Render Distance Widget"),
        Node { justify_self: JustifySelf::Start, ..default() },
        children![
            widget::button_small("-", lower_render_distance),
            (
                Name::new("Current Render Distance"),
                Node { padding: UiRect::horizontal(Val::Px(10.0)), justify_content: JustifyContent::Center, ..default() },
                children![(widget::label(""), RenderDistanceLabel)],
            ),
            widget::button_small("+", raise_render_distance),
        ],
    )
}

fn coords_widget() -> impl Bundle {
    (
        Name::new("Coords Widget"),
        Node { justify_self: JustifySelf::Start, ..default() },
        children![
            widget::button_small("-", toggle_coords),
            (
                Name::new("Current Coords"),
                Node { padding: UiRect::horizontal(Val::Px(10.0)), justify_content: JustifyContent::Center, ..default() },
                children![(widget::label(""), CoordsLabel)],
            ),
        ],
    )
}

const MIN_VOLUME: f32 = 0.0;
const MAX_VOLUME: f32 = 3.0;

fn lower_global_volume(_: On<Pointer<Click>>, mut global_volume: ResMut<GlobalVolume>) {
    let linear = (global_volume.volume.to_linear() - 0.1).max(MIN_VOLUME);
    global_volume.volume = Volume::Linear(linear);
}

fn raise_global_volume(_: On<Pointer<Click>>, mut global_volume: ResMut<GlobalVolume>) {
    let linear = (global_volume.volume.to_linear() + 0.1).min(MAX_VOLUME);
    global_volume.volume = Volume::Linear(linear);
}

fn lower_sensitivity(_: On<Pointer<Click>>, mut settings: ResMut<Settings>) {
    settings.mouse_sensitivity = (settings.mouse_sensitivity - 0.1).max(0.1);
}

fn raise_sensitivity(_: On<Pointer<Click>>, mut settings: ResMut<Settings>) {
    settings.mouse_sensitivity = (settings.mouse_sensitivity + 0.1).min(5.0);
}

fn lower_fov(_: On<Pointer<Click>>, mut settings: ResMut<Settings>) {
    settings.fov = (settings.fov - 5.0).max(60.0);
}

fn raise_fov(_: On<Pointer<Click>>, mut settings: ResMut<Settings>) {
    settings.fov = (settings.fov + 5.0).min(120.0);
}

fn lower_render_distance(_: On<Pointer<Click>>, mut settings: ResMut<Settings>) {
    settings.render_distance = (settings.render_distance - 1).max(2);
}

fn raise_render_distance(_: On<Pointer<Click>>, mut settings: ResMut<Settings>) {
    settings.render_distance = (settings.render_distance + 1).min(16);
}

fn toggle_coords(_: On<Pointer<Click>>, mut settings: ResMut<Settings>) {
    settings.show_coordinates = !settings.show_coordinates;
}

#[derive(Component, Reflect)]
#[reflect(Component)]
struct GlobalVolumeLabel;

#[derive(Component, Reflect)]
#[reflect(Component)]
struct SensitivityLabel;

#[derive(Component, Reflect)]
#[reflect(Component)]
struct FovLabel;

#[derive(Component, Reflect)]
#[reflect(Component)]
struct RenderDistanceLabel;

#[derive(Component, Reflect)]
#[reflect(Component)]
struct CoordsLabel;

fn update_global_volume_label(
    global_volume: Res<GlobalVolume>,
    mut label: Single<&mut Text, With<GlobalVolumeLabel>>,
) {
    let percent = 100.0 * global_volume.volume.to_linear();
    label.0 = format!("{percent:3.0}%");
}

fn update_sensitivity_label(
    settings: Res<Settings>,
    mut label: Single<&mut Text, With<SensitivityLabel>>,
) {
    label.0 = format!("{:.1}", settings.mouse_sensitivity);
}

fn update_fov_label(
    settings: Res<Settings>,
    mut label: Single<&mut Text, With<FovLabel>>,
) {
    label.0 = format!("{:.0}", settings.fov);
}

fn update_render_distance_label(
    settings: Res<Settings>,
    mut label: Single<&mut Text, With<RenderDistanceLabel>>,
) {
    label.0 = format!("{}", settings.render_distance);
}

fn update_coords_label(
    settings: Res<Settings>,
    mut label: Single<&mut Text, With<CoordsLabel>>,
) {
    label.0 = if settings.show_coordinates { "On" } else { "Off" }.to_string();
}

fn go_back_on_click(
    _: On<Pointer<Click>>,
    screen: Res<State<Screen>>,
    mut next_menu: ResMut<NextState<Menu>>,
) {
    next_menu.set(if screen.get() == &Screen::Title {
        Menu::Main
    } else {
        Menu::Pause
    });
}

fn go_back(screen: Res<State<Screen>>, mut next_menu: ResMut<NextState<Menu>>) {
    next_menu.set(if screen.get() == &Screen::Title {
        Menu::Main
    } else {
        Menu::Pause
    });
}
