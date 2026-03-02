//! Compute shaders use the GPU for computing arbitrary information, that may be independent of what
//! is rendered to the screen.

use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::{
        Render, RenderApp, RenderStartup, RenderSystems, extract_resource::{ExtractResource, ExtractResourcePlugin}, render_asset::RenderAssets, render_graph::{self, RenderGraph, RenderLabel}, render_resource::{
            binding_types::{texture_storage_2d, uniform_buffer},
            *,
        }, renderer::{RenderContext, RenderDevice, RenderQueue}, texture::GpuImage
    },
    shader::PipelineCacheError, window::PrimaryWindow,
};
use std::borrow::Cow;

/// This example uses a shader source file from the assets subdirectory
const SHADER_ASSET_PATH: &str = "compute_shader_sand.wgsl";

const DISPLAY_FACTOR: u32 = 4;
const SIZE: UVec2 = UVec2::new(1280 / DISPLAY_FACTOR, 720 / DISPLAY_FACTOR);
const WORKGROUP_SIZE: u32 = 8;

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::BLACK))
        .add_plugins((
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        resolution: (SIZE * DISPLAY_FACTOR).into(),
                        // uncomment for unthrottled FPS
                        // present_mode: bevy::window::PresentMode::AutoNoVsync,
                        ..default()
                    }),
                    ..default()
                })
                .set(ImagePlugin::default_nearest()),
            SandComputePlugin,
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, (
            switch_textures,
            handle_mouse_clicks,
        ))
        .run();
}

fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let mut image = Image::new_target_texture(SIZE.x, SIZE.y, TextureFormat::Rgba32Float);
    image.asset_usage = RenderAssetUsages::RENDER_WORLD;
    image.texture_descriptor.usage =
        TextureUsages::COPY_DST | TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING;
    let image0 = images.add(image.clone());
    let image1 = images.add(image);
    let mut metadata = Image::new_target_texture(SIZE.x, SIZE.y, TextureFormat::R32Uint);
    metadata.asset_usage = RenderAssetUsages::RENDER_WORLD;
    metadata.texture_descriptor.usage =
        TextureUsages::COPY_DST | TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING;
    let metadata0 = images.add(metadata.clone());
    let metadata1 = images.add(metadata);

    commands.spawn((
        Sprite {
            image: image0.clone(),
            custom_size: Some(SIZE.as_vec2()),
            ..default()
        },
        Transform::from_scale(Vec3::splat(DISPLAY_FACTOR as f32)),
    ));
    commands.spawn(Camera2d);

    commands.insert_resource(SandImages {
        texture_a: image0,
        texture_b: image1,
        metadata_a: metadata0,
        metadata_b: metadata1,
    });

    commands.insert_resource(SandUniforms {
        click_position: IVec2::new(-1, -1),
    });
}

// Switch texture to display every frame to show the one that was written to most recently.
fn switch_textures(images: Res<SandImages>, mut sprite: Single<&mut Sprite>) {
    if sprite.image == images.texture_a {
        sprite.image = images.texture_b.clone();
    } else {
        sprite.image = images.texture_a.clone();
    }
}

fn handle_mouse_clicks(
    mut uniforms: ResMut<SandUniforms>,
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
) {
    uniforms.click_position = IVec2::new(-1, -1);
    if let Ok(window) = windows.single()
    && let Some(cursor_position) = window.cursor_position()
    && let Ok((camera, camera_transform)) = camera_query.single()
    && let Ok(world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_position) {
        let texture_x = ((world_pos.x / DISPLAY_FACTOR as f32) + SIZE.x as f32 / 2.0)
            .clamp(0.0, SIZE.x as f32 - 1.0) as i32;
        let texture_y = (SIZE.y as f32 - 1.0 - ((world_pos.y / DISPLAY_FACTOR as f32) + SIZE.y as f32 / 2.0))
            .clamp(0.0, SIZE.y as f32 - 1.0) as i32;

        if texture_x >= 0
            && texture_x < i32::try_from(SIZE.x).unwrap_or(i32::MAX)
            && texture_y >= 0
            && texture_y < i32::try_from(SIZE.y).unwrap_or(i32::MAX)
            && mouse_button_input.pressed(MouseButton::Left)
        {
            uniforms.click_position = IVec2::new(texture_x, texture_y);
        }
    }
}

struct SandComputePlugin;

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct SandLabel;

impl Plugin for SandComputePlugin {
    fn build(&self, app: &mut App) {
        // Extract the image resource from the main world into the render world
        // for operation on by the compute shader and display on the sprite.
        app.add_plugins((
            ExtractResourcePlugin::<SandImages>::default(),
            ExtractResourcePlugin::<SandUniforms>::default(),
        ));
        let render_app = app.sub_app_mut(RenderApp);
        render_app
            .add_systems(RenderStartup, init_sand_pipeline)
            .add_systems(
                Render,
                prepare_bind_group.in_set(RenderSystems::PrepareBindGroups),
            );

        let mut render_graph = render_app.world_mut().resource_mut::<RenderGraph>();
        render_graph.add_node(SandLabel, SandNode::default());
        render_graph.add_node_edge(SandLabel, bevy::render::graph::CameraDriverLabel);
    }
}

#[derive(Resource, Clone, ExtractResource)]
struct SandImages {
    texture_a: Handle<Image>,
    texture_b: Handle<Image>,
    metadata_a: Handle<Image>,
    metadata_b: Handle<Image>,
}

#[derive(Resource, Clone, ExtractResource, ShaderType)]
struct SandUniforms {
    click_position: IVec2,
}

#[derive(Resource)]
struct SandImageBindGroups([BindGroup; 2]);

fn prepare_bind_group(
    mut commands: Commands,
    pipeline: Res<SandPipeline>,
    gpu_images: Res<RenderAssets<GpuImage>>,
    sand_images: Res<SandImages>,
    sand_uniforms: Res<SandUniforms>,
    render_device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
) {
    let view_a = gpu_images.get(&sand_images.texture_a).unwrap();
    let view_b = gpu_images.get(&sand_images.texture_b).unwrap();
    let metadata_a = gpu_images.get(&sand_images.metadata_a).unwrap();
    let metadata_b = gpu_images.get(&sand_images.metadata_b).unwrap();

    // Uniform buffer is used here to demonstrate how to set up a uniform in a compute shader
    // Alternatives such as storage buffers or push constants may be more suitable for your use case
    let mut uniform_buffer = UniformBuffer::from(sand_uniforms.into_inner());
    uniform_buffer.write_buffer(&render_device, &queue);

    let bind_group_0 = render_device.create_bind_group(
        None,
        &pipeline.texture_bind_group_layout,
        &BindGroupEntries::sequential((
            &view_a.texture_view,
            &view_b.texture_view,
            &metadata_a.texture_view,
            &metadata_b.texture_view,
            &uniform_buffer,
        )),
    );
    let bind_group_1 = render_device.create_bind_group(
        None,
        &pipeline.texture_bind_group_layout,
        &BindGroupEntries::sequential((
            &view_b.texture_view,
            &view_a.texture_view,
            &metadata_b.texture_view,
            &metadata_a.texture_view,
            &uniform_buffer,
        )),
    );
    commands.insert_resource(SandImageBindGroups([bind_group_0, bind_group_1]));
}

#[derive(Resource)]
struct SandPipeline {
    texture_bind_group_layout: BindGroupLayout,
    init_pipeline: CachedComputePipelineId,
    update_pipeline: CachedComputePipelineId,
}

fn init_sand_pipeline(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    asset_server: Res<AssetServer>,
    pipeline_cache: Res<PipelineCache>,
) {
    let texture_bind_group_layout = render_device.create_bind_group_layout(
        "SandImages",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::COMPUTE,
            (
                texture_storage_2d(TextureFormat::Rgba32Float, StorageTextureAccess::ReadOnly),
                texture_storage_2d(TextureFormat::Rgba32Float, StorageTextureAccess::WriteOnly),
                texture_storage_2d(TextureFormat::R32Uint, StorageTextureAccess::ReadOnly),
                texture_storage_2d(TextureFormat::R32Uint, StorageTextureAccess::WriteOnly),
                uniform_buffer::<SandUniforms>(false),
            ),
        ),
    );
    let shader = asset_server.load(SHADER_ASSET_PATH);
    let init_pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
        layout: vec![texture_bind_group_layout.clone()],
        shader: shader.clone(),
        entry_point: Some(Cow::from("init")),
        ..default()
    });
    let update_pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
        layout: vec![texture_bind_group_layout.clone()],
        shader,
        entry_point: Some(Cow::from("update")),
        ..default()
    });

    commands.insert_resource(SandPipeline {
        texture_bind_group_layout,
        init_pipeline,
        update_pipeline,
    });
}

enum SandState {
    Loading,
    Init,
    Update(usize),
}

struct SandNode {
    state: SandState,
}

impl Default for SandNode {
    fn default() -> Self {
        Self {
            state: SandState::Loading,
        }
    }
}

impl render_graph::Node for SandNode {
    fn update(&mut self, world: &mut World) {
        let pipeline = world.resource::<SandPipeline>();
        let pipeline_cache = world.resource::<PipelineCache>();

        // if the corresponding pipeline has loaded, transition to the next stage
        match self.state {
            SandState::Loading => {
                match pipeline_cache.get_compute_pipeline_state(pipeline.init_pipeline) {
                    CachedPipelineState::Ok(_) => {
                        self.state = SandState::Init;
                    }
                    // If the shader hasn't loaded yet, just wait.
                    CachedPipelineState::Err(PipelineCacheError::ShaderNotLoaded(_)) => {}
                    CachedPipelineState::Err(err) => {
                        panic!("Initializing assets/{SHADER_ASSET_PATH}:\n{err}")
                    }
                    _ => {}
                }
            }
            SandState::Init => {
                if let CachedPipelineState::Ok(_) =
                    pipeline_cache.get_compute_pipeline_state(pipeline.update_pipeline)
                {
                    self.state = SandState::Update(1);
                }
            }
            SandState::Update(0) => {
                self.state = SandState::Update(1);
            }
            SandState::Update(1) => {
                self.state = SandState::Update(0);
            }
            SandState::Update(_) => unreachable!(),
        }
    }

    fn run(
        &self,
        _graph: &mut render_graph::RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), render_graph::NodeRunError> {
        let bind_groups = &world.resource::<SandImageBindGroups>().0;
        let pipeline_cache = world.resource::<PipelineCache>();
        let pipeline = world.resource::<SandPipeline>();

        let mut pass = render_context
            .command_encoder()
            .begin_compute_pass(&ComputePassDescriptor::default());

        // select the pipeline based on the current state
        match self.state {
            SandState::Loading => {}
            SandState::Init => {
                let init_pipeline = pipeline_cache
                    .get_compute_pipeline(pipeline.init_pipeline)
                    .unwrap();
                pass.set_bind_group(0, &bind_groups[0], &[]);
                pass.set_pipeline(init_pipeline);
                pass.dispatch_workgroups(SIZE.x / WORKGROUP_SIZE, SIZE.y / WORKGROUP_SIZE, 1);
            }
            SandState::Update(index) => {
                let update_pipeline = pipeline_cache
                    .get_compute_pipeline(pipeline.update_pipeline)
                    .unwrap();
                pass.set_bind_group(0, &bind_groups[index], &[]);
                pass.set_pipeline(update_pipeline);
                pass.dispatch_workgroups(SIZE.x / WORKGROUP_SIZE, SIZE.y / WORKGROUP_SIZE, 1);
            }
        }

        Ok(())
    }
}