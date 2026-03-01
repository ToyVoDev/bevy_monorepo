use bevy::prelude::*;

pub mod first_person;
pub mod third_person;

#[derive(Component)]
pub struct OnCameraUIReticle;

#[derive(Component)]
pub struct OnCameraUIInteract;
