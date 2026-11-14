// Includes another library: an include path is relative to `assets/`, not to this file.
#include "shaders/lib/hash.wgsl"

fn value_noise3(p: vec3<f32>) -> f32 {
    let cell = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);

    let c000 = hash31(cell + vec3<f32>(0.0, 0.0, 0.0));
    let c100 = hash31(cell + vec3<f32>(1.0, 0.0, 0.0));
    let c010 = hash31(cell + vec3<f32>(0.0, 1.0, 0.0));
    let c110 = hash31(cell + vec3<f32>(1.0, 1.0, 0.0));
    let c001 = hash31(cell + vec3<f32>(0.0, 0.0, 1.0));
    let c101 = hash31(cell + vec3<f32>(1.0, 0.0, 1.0));
    let c011 = hash31(cell + vec3<f32>(0.0, 1.0, 1.0));
    let c111 = hash31(cell + vec3<f32>(1.0, 1.0, 1.0));

    let bottom = mix(mix(c000, c100, u.x), mix(c010, c110, u.x), u.y);
    let top = mix(mix(c001, c101, u.x), mix(c011, c111, u.x), u.y);

    return mix(bottom, top, u.z);
}

fn fbm3(p: vec3<f32>) -> f32 {
    var value = 0.0;
    var amplitude = 0.5;
    var position = p;

    for (var i = 0; i < 4; i++) {
        value += amplitude * value_noise3(position);
        position *= 2.03;
        amplitude *= 0.5;
    }

    return value / 0.9375;
}
