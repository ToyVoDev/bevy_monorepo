// Components, Resources & States
use avian3d::prelude::RigidBody;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Resource)]
pub struct Cubemap {
    pub is_loaded: bool,
    pub image_handle: Handle<Image>,
}

#[derive(Component)]
#[require(Transform)]
pub struct Player;

#[derive(Clone, Copy, Default, Eq, PartialEq, Debug, Hash, States)]
pub enum PlayerState {
    #[default]
    None,
    Id(Entity),
}

#[derive(Clone, Copy, Default, Eq, PartialEq, Debug, Hash, States)]
pub enum GameState {
    #[default]
    Menu,
    Game,
}

// One of the two settings that can be set through the menu. It will be a resource in the app
#[derive(Resource, Debug, Component, PartialEq, Eq, Clone, Copy)]
pub enum DisplayQuality {
    Low,
    Medium,
    High,
}

// One of the two settings that can be set through the menu. It will be a resource in the app
#[derive(Resource, Debug, Component, PartialEq, Eq, Clone, Copy)]
pub struct Volume(pub u32);

// B stands for blender
#[derive(Serialize, Deserialize, Debug)]
pub struct BMeshExtra {
    pub collider: BCollider,
    pub rigid_body: BRigidBody,
    pub cube_size: Option<Vec3>,
    pub sphere_radius: Option<f32>,
}

// B stands for blender
#[derive(Serialize, Deserialize, Debug)]
pub enum BCollider {
    TrimeshFromMesh,
    Cuboid,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum BRigidBody {
    Static,
    Dynamic,
}

impl From<BRigidBody> for RigidBody {
    fn from(value: BRigidBody) -> Self {
        match value {
            BRigidBody::Dynamic => RigidBody::Dynamic,
            BRigidBody::Static => RigidBody::Static,
        }
    }
}
