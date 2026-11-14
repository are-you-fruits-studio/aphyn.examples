// Vertex displacement on a grid, animated by the `time` input that the demo writes into the
// material every frame. All tiles share one material and one mesh, so they are drawn as instances;
// the only per-instance data is the model matrix, so each tile takes its wave phase from its
// world position.
#include "shaders/lib/globals.wgsl"
#include "aphyn/pbr.wgsl"

struct WavesMaterial {
    color: vec4<f32>,
    time: f32,
}

@group(1) @binding(0) var<uniform> material: WavesMaterial;

const AMPLITUDE = 0.05;
const FREQUENCY_X = 11.0;
const FREQUENCY_Z = 8.0;

struct VertexOutput {
    @builtin(position) position_clip: vec4<f32>,
    @location(0) normal_world: vec3<f32>,
    @location(1) position_world: vec3<f32>,
};

@vertex
fn vs_main(vertex: VertexAttributes, instance_raw: InstanceRawAttributes) -> VertexOutput {
    let instance = map_instance_data(instance_raw);
    var out: VertexOutput;

    let phase = dot(instance.local_to_world[3].xyz, vec3<f32>(2.1, 3.7, 5.3));
    let wave_x = vertex.position.x * FREQUENCY_X + material.time * 2.0 + phase;
    let wave_z = vertex.position.z * FREQUENCY_Z - material.time * 1.4 + phase * 1.7;

    let height = AMPLITUDE * (sin(wave_x) + sin(wave_z));
    let slope_x = AMPLITUDE * FREQUENCY_X * cos(wave_x);
    let slope_z = AMPLITUDE * FREQUENCY_Z * cos(wave_z);

    let position_local = vec4<f32>(vertex.position.x, height, vertex.position.z, 1.0);
    let position_world = instance.local_to_world * position_local;

    out.position_clip = global_data.matrix_world_to_clip * position_world;
    out.normal_world = (instance.local_to_world * vec4<f32>(-slope_x, 1.0, -slope_z, 0.0)).xyz;
    out.position_world = position_world.xyz;

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let normal = normalize(in.normal_world);
    let view_dir = normalize(global_data.camera_position_world - in.position_world);

    var color = material.color.xyz * 0.05;

    for (var i: u32 = 0; i < global_data.lights_count; i++) {
        let light = light_generic_to_fragment(lights[i], in.position_world);

        color += cook_torrance_brdf(
            normal,
            view_dir,
            material.color.xyz,
            0.0,
            0.3,
            light.direction_src,
            light.color
        );
    }

    return vec4<f32>(color, 1.0);
}
