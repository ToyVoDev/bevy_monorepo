@group(0) @binding(0) var input: texture_storage_2d<rgba32float, read>;

@group(0) @binding(1) var output: texture_storage_2d<rgba32float, write>;

@group(0) @binding(2) var metadata_input: texture_storage_2d<r32uint, read>;

@group(0) @binding(3) var metadata_output: texture_storage_2d<r32uint, write>;

@group(0) @binding(4) var<uniform> config: SandUniforms;

const BACKGROUND_COLOR: vec4<f32> = vec4<f32>(0.0, 0.0, 0.0, 1.0);
const SAND_COLOR: vec4<f32> = vec4<f32>(0.8745098, 0.75686275, 0.38823529, 1.0);
const BACKGROUND_ID: u32 = 0u;
const SAND_ID: u32 = 1u;

struct SandUniforms {
    click_position: vec2<i32>,
}

@compute @workgroup_size(8, 8, 1)
fn init(@builtin(global_invocation_id) invocation_id: vec3<u32>, @builtin(num_workgroups) num_workgroups: vec3<u32>) {
    let location = vec2<i32>(i32(invocation_id.x), i32(invocation_id.y));
    textureStore(output, location, BACKGROUND_COLOR);
    textureStore(metadata_output, location, vec4<u32>(BACKGROUND_ID, 0u, 0u, 0u));
}

@compute @workgroup_size(8, 8, 1)
fn update(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
    let location = vec2<i32>(i32(invocation_id.x), i32(invocation_id.y));
    let metadata = textureLoad(metadata_input, location);
    let pixel_below = textureLoad(input, location + vec2<i32>(0, 1));
    let pixel_below_metadata = textureLoad(metadata_input, location + vec2<i32>(0, 1));

    if (location.x == config.click_position.x && location.y == config.click_position.y) {
        textureStore(output, location, SAND_COLOR);
        textureStore(metadata_output, location, vec4<u32>(SAND_ID, 0u, 0u, 0u));
        if (pixel_below_metadata.r == BACKGROUND_ID) {
            textureStore(output, location + vec2<i32>(0, 1), SAND_COLOR);
            textureStore(metadata_output, location + vec2<i32>(0, 1), vec4<u32>(SAND_ID, 0u, 0u, 0u));
        }
        return;
    }

    if (metadata.r == SAND_ID && pixel_below_metadata.r == BACKGROUND_ID) {
        textureStore(output, location + vec2<i32>(0, 1), SAND_COLOR);
        textureStore(metadata_output, location + vec2<i32>(0, 1), vec4<u32>(SAND_ID, 0u, 0u, 0u));
        return;
    }

    textureStore(output, location, BACKGROUND_COLOR);
    textureStore(metadata_output, location, vec4<u32>(BACKGROUND_ID, 0u, 0u, 0u));
}