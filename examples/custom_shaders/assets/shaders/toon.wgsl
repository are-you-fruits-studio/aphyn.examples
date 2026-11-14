// Cel shading with a rim: the light from every entry of the engine's `lights` buffer is summed and
// cut into three hard bands, and the rim lights up where the surface turns away from the camera.
#include "shaders/lib/globals.wgsl"
#include "shaders/lib/lights.wgsl"

struct ToonMaterial {
    color: vec4<f32>,
}

@group(1) @binding(0) var<uniform> material: ToonMaterial;

const RIM_COLOR = vec3<f32>(1.0, 0.95, 0.8);

struct VertexOutput {
    @builtin(position) position_clip: vec4<f32>,
    @location(0) normal_world: vec3<f32>,
    @location(1) position_world: vec3<f32>,
};

@vertex
fn vs_main(vertex: VertexAttributes, instance_raw: InstanceRawAttributes) -> VertexOutput {
    let instance = map_instance_data(instance_raw);
    var out: VertexOutput;

    let position_world = instance.local_to_world * vec4<f32>(vertex.position, 1.0);

    out.position_clip = global_data.matrix_world_to_clip * position_world;
    out.normal_world = (instance.local_to_world * vec4<f32>(vertex.normal, 0.0)).xyz;
    out.position_world = position_world.xyz;

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let normal = normalize(in.normal_world);
    let view_dir = normalize(global_data.camera_position_world - in.position_world);

    let brightness = diffuse_brightness(normal, in.position_world);
    let band = ceil(clamp(brightness, 0.0, 1.0) * 3.0) / 3.0;
    let rim = step(0.75, 1.0 - max(dot(normal, view_dir), 0.0));
    let color = material.color.xyz * (0.12 + 0.88 * band);

    return vec4<f32>(mix(color, RIM_COLOR, rim), 1.0);
}
