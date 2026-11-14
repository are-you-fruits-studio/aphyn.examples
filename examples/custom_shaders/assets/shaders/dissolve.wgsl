// Noise-driven `discard` with a glowing edge. A discarded fragment writes neither color nor depth,
// so the holes show the core sphere inside. The edge color is far above 1.0, which the HDR target
// keeps, so the bloom pass picks it up.
#include "shaders/lib/globals.wgsl"
#include "shaders/lib/noise.wgsl"
#include "shaders/lib/lights.wgsl"

struct DissolveMaterial {
    color: vec4<f32>,
    edge_color: vec4<f32>,
    time: f32,
}

@group(1) @binding(0) var<uniform> material: DissolveMaterial;

const EDGE_WIDTH = 0.05;
const EDGE_BRIGHTNESS = 9.0;

struct VertexOutput {
    @builtin(position) position_clip: vec4<f32>,
    @location(0) normal_world: vec3<f32>,
    @location(1) position_world: vec3<f32>,
    @location(2) position_local: vec3<f32>,
};

@vertex
fn vs_main(vertex: VertexAttributes, instance_raw: InstanceRawAttributes) -> VertexOutput {
    let instance = map_instance_data(instance_raw);
    var out: VertexOutput;

    let position_world = instance.local_to_world * vec4<f32>(vertex.position, 1.0);

    out.position_clip = global_data.matrix_world_to_clip * position_world;
    out.normal_world = (instance.local_to_world * vec4<f32>(vertex.normal, 0.0)).xyz;
    out.position_world = position_world.xyz;
    out.position_local = vertex.position;

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let threshold = mix(0.25, 0.75, 0.5 + 0.5 * sin(material.time * 0.8));
    let noise = fbm3(in.position_local * 5.0);

    if (noise < threshold) {
        discard;
    }

    if (noise < threshold + EDGE_WIDTH) {
        return vec4<f32>(material.edge_color.xyz * EDGE_BRIGHTNESS, 1.0);
    }

    let normal = normalize(in.normal_world);
    let brightness = 0.1 + diffuse_brightness(normal, in.position_world);

    return vec4<f32>(material.color.xyz * brightness, 1.0);
}
