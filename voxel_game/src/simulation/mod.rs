pub mod debris;

use bevy::prelude::*;

pub struct SimulationPlugin;
impl Plugin for SimulationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, debris::setup_debris_assets)
           .add_systems(Update, (
               debris::tick_debris,
               debris::solidify_resting_debris,
           ));
    }
}
