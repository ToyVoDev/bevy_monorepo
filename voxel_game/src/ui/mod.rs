pub mod asset_tracking;
pub mod audio;
pub mod menus;
pub mod screens;
pub mod theme;

use bevy::prelude::*;

pub fn plugin(app: &mut App) {
    app.add_plugins((
        asset_tracking::plugin,
        audio::plugin,
        menus::plugin,
        screens::plugin,
        theme::plugin,
    ));
}
