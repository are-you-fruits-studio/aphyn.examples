// Shows one vertex attribute as a color: `mode` 0 is the world normal, 1 the uv, 2 the world tangent.
// The three debug materials share this shader and differ only in their `mode` value.
#include "shaders/lib/globals.wgsl"

struct DebugMaterial {
    mode: f32,
}

@group(1) @binding(0) var<uniform> material: DebugMaterial;

struct VertexOutput {
    @builtin(position) position_clip: vec4<f32>,
    @location(0) normal_world: vec3<f32>,
    @location(1) tangent_world: vec3<f32>,
    @location(2) uv: vec2<f32>,
};

@vertex
fn vs_main(vertex: VertexAttributes, instance_raw: InstanceRawAttributes) -> VertexOutput {
    let instance = map_instance_data(instance_raw);
    var out: VertexOutput;

    out.position_clip = global_data.matrix_world_to_clip * instance.local_to_world * vec4<f32>(vertex.position, 1.0);
    out.normal_world = (instance.local_to_world * vec4<f32>(vertex.normal, 0.0)).xyz;
    out.tangent_world = (instance.local_to_world * vec4<f32>(vertex.tangent.xyz, 0.0)).xyz;
    out.uv = vertex.uv;

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var color: vec3<f32>;

    switch u32(material.mode) {
        case 1u: {
            color = vec3<f32>(in.uv, 0.0);
        }
        case 2u: {
            color = normalize(in.tangent_world) * 0.5 + 0.5;
        }
        default: {
            color = normalize(in.normal_world) * 0.5 + 0.5;
        }
    }

    return vec4<f32>(color, 1.0);
}
