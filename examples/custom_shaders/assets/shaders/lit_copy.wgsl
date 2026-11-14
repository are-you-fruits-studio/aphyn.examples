// A copy of the engine's standard shader with its `LIT` branches written out, because a shader
// loaded from an asset file is composed without defines. It uses the engine's standard bindings
// and is wrapped in the engine's standard material template, so it takes the same inputs as the
// built-in lit material and should draw exactly the same picture.
#include "aphyn/standard_bindings.wgsl"
#include "aphyn/pbr.wgsl"

struct VertexOutput {
    @builtin(position) position_clip: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) normal_world: vec3<f32>,
    @location(2) tangent_world: vec4<f32>,
    @location(3) position_world: vec3<f32>,
    @location(4) uv: vec2<f32>,
};

@vertex
fn vs_main(vertex: VertexAttributes, instance_raw: InstanceRawAttributes) -> VertexOutput {
    let instance = map_instance_data(instance_raw);
    var out: VertexOutput;

    let position_world = instance.local_to_world * vec4<f32>(vertex.position, 1.0);

    out.position_clip = global_data.matrix_world_to_clip * position_world;
    out.color = vertex.color * material_data.color;
    out.normal_world = (instance.local_to_world * vec4<f32>(vertex.normal, 0.0)).xyz;
    out.tangent_world = vec4<f32>((instance.local_to_world * vec4<f32>(vertex.tangent.xyz, 0.0)).xyz, vertex.tangent.w);
    out.position_world = position_world.xyz;
    out.uv = vertex.uv;

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let base = in.color * textureSample(color_texture, color_sampler, in.uv);

    if (base.w < material_data.alpha_threshold) {
        discard;
    }

    let emission = material_data.emission_color.xyz * material_data.emission_color.w * textureSample(emission_texture, emission_sampler, in.uv).xyz;
    var color = emission;

    let normal_tangent_space = textureSample(normal_texture, normal_sampler, in.uv).xyz * 2.0 - 1.0;
    let tbn = create_matrix_tbn(in.normal_world, in.tangent_world);
    let normal_world = normalize(tbn * normal_tangent_space);

    let view_dir = normalize(global_data.camera_position_world - in.position_world);
    let metallic = material_data.metallic * textureSample(metallic_texture, metallic_sampler, in.uv).x;
    let roughness = material_data.roughness * textureSample(roughness_texture, roughness_sampler, in.uv).x;

    for (var i: u32 = 0; i < global_data.lights_count; i++) {
        let light = light_generic_to_fragment(lights[i], in.position_world);

        color += cook_torrance_brdf(
            normal_world,
            view_dir,
            base.xyz,
            metallic,
            roughness,
            light.direction_src,
            light.color
        );
    }

    return vec4<f32>(color, 1.0);
}
