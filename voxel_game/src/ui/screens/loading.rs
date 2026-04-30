//! A loading screen during which game assets are loaded if necessary.

use bevy::prelude::*;

use crate::ui::{asset_tracking::ResourceHandles, screens::Screen, theme::widget};

pub(super) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(Screen::Loading), spawn_loading_screen);
    app.add_systems(
        Update,
        enter_gameplay_screen.run_if(in_state(Screen::Loading).and(all_assets_loaded)),
    );
}

fn spawn_loading_screen(mut commands: Commands) {
    commands.spawn((
        widget::ui_root("Loading Screen"),
        DespawnOnExit(Screen::Loading),
        children![widget::label("Loading...")],
    ));
}

fn enter_gameplay_screen(mut next_screen: ResMut<NextState<Screen>>) {
    next_screen.set(Screen::WorldLoading);
}

fn all_assets_loaded(resource_handles: Res<ResourceHandles>) -> bool {
    resource_handles.is_all_done()
}
