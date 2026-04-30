//! The game's main screen states and transitions between them.

mod gameplay;
mod loading;
mod splash;
mod title;
mod world_loading;

use bevy::prelude::*;

pub(super) fn plugin(app: &mut App) {
    app.init_state::<Screen>();

    app.add_plugins((
        gameplay::plugin,
        loading::plugin,
        splash::plugin,
        title::plugin,
        world_loading::plugin,
    ));
}

/// The game's main screen states.
#[derive(States, Copy, Clone, Eq, PartialEq, Hash, Debug, Default)]
pub enum Screen {
    #[default]
    Splash,
    Title,
    Loading,
    WorldLoading,
    Gameplay,
}
