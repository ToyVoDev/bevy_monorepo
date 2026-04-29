use bevy::prelude::*;
use crate::config::{CHUNK_SIZE, VOXEL_SIZE};
use crate::player::interaction::TargetedVoxel;

pub fn spawn_hud(mut commands: Commands) {
    // Full-screen flex container, centers children
    commands.spawn(Node {
        width: Val::Percent(100.0),
        height: Val::Percent(100.0),
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        position_type: PositionType::Absolute,
        ..default()
    }).with_children(|root| {
        // Zero-size anchor at screen center
        root.spawn(Node::default()).with_children(|center| {
            // Horizontal bar
            center.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Px(20.0),
                    height: Val::Px(2.0),
                    left: Val::Px(-10.0),
                    top: Val::Px(-1.0),
                    ..default()
                },
                BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.85)),
            ));
            // Vertical bar
            center.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Px(2.0),
                    height: Val::Px(20.0),
                    left: Val::Px(-1.0),
                    top: Val::Px(-10.0),
                    ..default()
                },
                BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.85)),
            ));
        });
    });
}

#[derive(Component)]
pub struct VoxelHighlight;

pub fn spawn_highlight(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let size = VOXEL_SIZE * 1.015; // slightly larger to sit on top of the voxel face
    commands.spawn((
        VoxelHighlight,
        Mesh3d(meshes.add(Cuboid::new(size, size, size))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgba(1.0, 0.85, 0.2, 0.35),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            double_sided: true,
            cull_mode: None,
            ..default()
        })),
        Transform::default(),
        Visibility::Hidden,
    ));
}

pub fn update_highlight(
    targeted: Res<TargetedVoxel>,
    mut query: Query<(&mut Transform, &mut Visibility), With<VoxelHighlight>>,
) {
    let Ok((mut transform, mut visibility)) = query.single_mut() else { return };

    match &targeted.0 {
        Some(hit) => {
            let n = CHUNK_SIZE as i32;
            transform.translation = Vec3::new(
                (hit.chunk.0 * n + hit.local.x as i32) as f32 * VOXEL_SIZE + VOXEL_SIZE * 0.5,
                (hit.chunk.1 * n + hit.local.y as i32) as f32 * VOXEL_SIZE + VOXEL_SIZE * 0.5,
                (hit.chunk.2 * n + hit.local.z as i32) as f32 * VOXEL_SIZE + VOXEL_SIZE * 0.5,
            );
            *visibility = Visibility::Visible;
        }
        None => {
            *visibility = Visibility::Hidden;
        }
    }
}
