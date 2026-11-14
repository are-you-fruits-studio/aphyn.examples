// Diffuse light summed over every entry of the engine's `lights` buffer: each light counts with its
// strongest color channel, weighted by how directly it falls on the surface.
#include "shaders/lib/globals.wgsl"
#include "aphyn/pbr.wgsl"

fn diffuse_brightness(normal: vec3<f32>, position_world: vec3<f32>) -> f32 {
    var brightness = 0.0;

    for (var i: u32 = 0; i < global_data.lights_count; i++) {
        let light = light_generic_to_fragment(lights[i], position_world);
        let strength = max(light.color.x, max(light.color.y, light.color.z));

        brightness += max(dot(normal, light.direction_src), 0.0) * strength;
    }

    return brightness;
}
