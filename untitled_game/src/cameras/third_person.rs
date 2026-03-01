use crate::cameras::{OnCameraUIInteract, OnCameraUIReticle};
use crate::menu::MenuState;
use crate::{despawn_screen, Player, PlayerState};
use crate::{pause_physics, unpause_physics};
use avian3d::prelude::*;
use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;
use bevy::window::CursorGrabMode;
use std::f32::consts::*;
use std::ops::Range;

#[derive(Debug, Component)]
#[require(Transform)]
pub struct CameraController {
    pub orbit_distance: f32,
    pub pitch_speed: f32,
    // Clamp pitch to this range
    pub pitch_range: Range<f32>,
    pub yaw_speed: f32,
    pub key_left: KeyCode,
    pub key_right: KeyCode,
    pub key_up: KeyCode,
    pub key_down: KeyCode,
    pub key_pause: KeyCode,
    pub key_interact: KeyCode,
    pub mouse_interact: MouseButton,
    pub walk_speed: f32,
    pub run_speed: f32,
    pub friction: f32,
    pub velocity: Vec3,
    pub key_run: KeyCode,
}

impl Default for CameraController {
    fn default() -> Self {
        Self {
            friction: 0.1,
            key_down: KeyCode::KeyS,
            key_left: KeyCode::KeyA,
            key_pause: KeyCode::Escape,
            key_right: KeyCode::KeyD,
            key_up: KeyCode::KeyW,
            key_run: KeyCode::ShiftLeft,
            key_interact: KeyCode::KeyE,
            mouse_interact: MouseButton::Left,
            orbit_distance: 10.0,
            // Limiting pitch stops some unexpected rotation
            pitch_range: -(FRAC_PI_2 - FRAC_1_PI)..FRAC_1_PI,
            pitch_speed: 0.003,
            run_speed: 15.0,
            walk_speed: 5.0,
            yaw_speed: 0.004,
            velocity: Vec3::ZERO,
        }
    }
}

fn camera_controller_update(
    query: Single<(&mut Transform, &mut CameraController), (With<Camera>, Without<Player>)>,
    accumulated_mouse_motion: Res<AccumulatedMouseMotion>,
    time: Res<Time>,
    key_input: Res<ButtonInput<KeyCode>>,
    mut window: Single<&mut Window>,
    mut player: Single<&mut Transform, With<Player>>,
    mut menu_state: ResMut<NextState<MenuState>>,
) {
    let (mut camera, mut controller) = query.into_inner();

    if key_input.pressed(controller.key_pause) {
        window.cursor_options.grab_mode = CursorGrabMode::None;
        window.cursor_options.visible = true;
        menu_state.set(MenuState::Main)
    }

    let dt = time.delta_secs();

    let mut axis_input = Vec3::ZERO;
    if key_input.pressed(controller.key_up) {
        axis_input.z += 1.0;
    }
    if key_input.pressed(controller.key_down) {
        axis_input.z -= 1.0;
    }
    if key_input.pressed(controller.key_right) {
        axis_input.x += 1.0;
    }
    if key_input.pressed(controller.key_left) {
        axis_input.x -= 1.0;
    }

    // Apply movement update
    if axis_input != Vec3::ZERO {
        let max_speed = if key_input.pressed(controller.key_run) {
            controller.run_speed
        } else {
            controller.walk_speed
        };
        controller.velocity = axis_input.normalize() * max_speed;
    } else {
        let friction = controller.friction.clamp(0.0, 1.0);
        controller.velocity *= 1.0 - friction;
        if controller.velocity.length_squared() < 1e-6 {
            controller.velocity = Vec3::ZERO;
        }
    }
    let forward = camera.forward();
    let forward = Dir3::new(Vec3::new(forward.x, 0., forward.z)).unwrap();
    let right = camera.right();
    player.translation += controller.velocity.x * dt * right
        + controller.velocity.y * dt * Vec3::Y
        + controller.velocity.z * dt * forward;

    let delta = accumulated_mouse_motion.delta;

    // Mouse motion is one of the few inputs that should not be multiplied by delta time,
    // as we are already receiving the full movement since the last frame was rendered. Multiplying
    // by delta time here would make the movement slower that it should be.
    let delta_pitch = delta.y * controller.pitch_speed;
    let delta_yaw = delta.x * controller.yaw_speed;

    // Obtain the existing pitch, yaw, and roll values from the transform.
    let (yaw, pitch, roll) = camera.rotation.to_euler(EulerRot::YXZ);

    // Establish the new yaw and pitch, preventing the pitch value from exceeding our limits.
    let pitch =
        (pitch + delta_pitch).clamp(controller.pitch_range.start, controller.pitch_range.end);
    let yaw = yaw - delta_yaw;
    camera.rotation = Quat::from_euler(EulerRot::YXZ, yaw, pitch, roll);

    let target = player.translation + Vec3::new(0., 1., 0.1);

    // Adjust the translation to maintain the correct orientation toward the orbit target.
    // In our example it's a static target, but this could easily be customized.
    camera.translation = target - camera.forward() * controller.orbit_distance;
}

fn camera_controller_setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands
        .spawn((
            Node {
                width: Val::Percent(100.),
                height: Val::Percent(100.),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            OnCameraUIReticle,
        ))
        .with_child((ImageNode::new(asset_server.load("reticle.png")),));
}

pub struct CameraControllerPlugin;

#[derive(Clone, Copy, Default, Eq, PartialEq, Debug, Hash, States)]
pub enum CameraUIState {
    Visible,
    #[default]
    Hidden,
}

impl Plugin for CameraControllerPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<CameraUIState>()
            .add_systems(
                OnEnter(MenuState::Disabled),
                (camera_controller_setup, unpause_physics),
            )
            .add_systems(
                OnExit(MenuState::Disabled),
                (despawn_screen::<OnCameraUIReticle>, pause_physics, hide_ui),
            )
            .add_systems(OnEnter(CameraUIState::Visible), interaction_display_setup)
            .add_systems(
                OnExit(CameraUIState::Visible),
                despawn_screen::<OnCameraUIInteract>,
            )
            .add_systems(
                Update,
                (camera_controller_update, print_hits).run_if(in_state(MenuState::Disabled)),
            );
    }
}

fn print_hits(
    camera: Single<(&mut Transform, &CameraController), With<Camera3d>>,
    key_input: Res<ButtonInput<KeyCode>>,
    mut camera_ui_state: ResMut<NextState<CameraUIState>>,
    spatial_query: SpatialQuery,
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    player_state: Res<State<PlayerState>>,
) {
    let (camera_transform, camera_controller) = camera.into_inner();
    // Ray origin and direction
    let origin = camera_transform.translation;
    let direction = camera_transform.forward();

    // Configuration for the ray cast
    let max_distance = 100.0;
    let solid = true;
    let filter = SpatialQueryFilter::default();

    // Cast ray and print first hit
    if let Some(first_hit) = spatial_query.cast_ray(origin, direction, max_distance, solid, &filter)
    {
        if let PlayerState::Id(player) = player_state.clone() {
            if first_hit.entity != player {
                camera_ui_state.set(CameraUIState::Visible);
                if key_input.just_pressed(camera_controller.key_interact) {
                    commands
                        .entity(first_hit.entity)
                        .insert(MeshMaterial3d(materials.add(Color::srgb_u8(0, 0, 0))));
                }
            } else {
                camera_ui_state.set(CameraUIState::Hidden);
            }
        } else {
            camera_ui_state.set(CameraUIState::Hidden);
        }
    } else {
        camera_ui_state.set(CameraUIState::Hidden);
    }
}

fn interaction_display_setup(mut commands: Commands, camera_controller: Single<&CameraController>) {
    commands
        .spawn((
            Node {
                width: Val::Percent(100.),
                height: Val::Percent(100.),
                align_items: AlignItems::End,
                justify_content: JustifyContent::End,
                ..default()
            },
            OnCameraUIInteract,
        ))
        .with_child(Text::new(format!(
            "Press {:?} to Interact",
            camera_controller.key_interact
        )));
}

fn hide_ui(mut camera_ui_state: ResMut<NextState<CameraUIState>>) {
    camera_ui_state.set(CameraUIState::Hidden);
}
