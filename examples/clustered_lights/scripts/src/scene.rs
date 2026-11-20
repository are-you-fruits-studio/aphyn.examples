use aphyn::*;

use crate::mesh;

// the floor spans [-AREA_HALF, AREA_HALF] on X and Z
const AREA_HALF: f32 = 48.0;
const OBJECTS_COUNT: u32 = 2000;
const LIGHTS_COUNT: u32 = 1024;
const COLORS: [[f32; 3]; 6] = [
    [0.80, 0.80, 0.80],
    [0.85, 0.35, 0.30],
    [0.30, 0.65, 0.40],
    [0.30, 0.45, 0.85],
    [0.90, 0.75, 0.35],
    [0.60, 0.40, 0.75],
];

// deterministic per (index, salt), so a run always builds the same scene
fn rand(index: u32, salt: u32) -> f32 {
    let mut x = index.wrapping_mul(0x9e37_79b9) ^ salt.wrapping_mul(0x85eb_ca6b) ^ 0x2545_f491;
    x ^= x >> 16;
    x = x.wrapping_mul(0x7feb_352d);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846c_a68b);
    x ^= x >> 16;
    x as f32 / u32::MAX as f32
}

fn rand_range(index: u32, salt: u32, min: f32, max: f32) -> f32 {
    min + (max - min) * rand(index, salt)
}

fn random_axis(index: u32, salt: u32) -> Vec3<f32> {
    let axis = Vec3::new(
        rand(index, salt) - 0.5,
        rand(index, salt + 1) - 0.5,
        rand(index, salt + 2) - 0.5,
    );
    if axis.length() < 1e-3 {
        Vec3::new(0.0, 1.0, 0.0)
    } else {
        axis.normalized()
    }
}

#[derive(Component)]
pub struct LightMotion {
    base: Vec3<f32>,
    amplitude: Vec3<f32>,
    frequency: Vec3<f32>,
    phase: Vec3<f32>,
}

pub fn spawn_camera(mut world: WorldDataMut) {
    let position = Vec3::new(0.0, 32.0, -68.0);
    let target = Vec3::new(0.0, 0.0, -4.0);

    let mut ec = world.entities_mut();
    let camera = ec.create_entity();
    ec.set_components(
        camera,
        (
            LocalTransform {
                position,
                rotation: Quat::look_rotation(target - position, Vec3::new(0.0, 1.0, 0.0)),
                scale: Vec3::splat(1.0),
            },
            CameraComponent(Camera {
                near: 0.1,
                far: 200.0,
                projection: CameraProjection::Perspective {
                    fov: 60_f32.to_radians(),
                },
            }),
        ),
    );
}

pub fn setup_scene(mut world: WorldDataMut) {
    let cube = mesh::upload(&mut world, mesh::make_cube());
    let sphere = mesh::upload(&mut world, mesh::make_sphere(24, 16));
    let materials = COLORS.map(|[r, g, b]| lit(&mut world, Vec4::new(r, g, b, 1.0), 0.6));
    let floor_material = lit(&mut world, Vec4::new(0.55, 0.55, 0.55, 1.0), 0.9);

    let mut ec = world.entities_mut();

    let floor = ec.create_entity();
    ec.set_components(
        floor,
        (
            LocalTransform {
                position: Vec3::new(0.0, -0.5, 0.0),
                rotation: Quat::IDENTITY,
                scale: Vec3::new(AREA_HALF * 2.0 + 4.0, 1.0, AREA_HALF * 2.0 + 4.0),
            },
            MeshComponent { mesh: cube.clone() },
            MaterialComponent {
                material: floor_material,
            },
        ),
    );

    for i in 0..OBJECTS_COUNT {
        let scale = rand_range(i, 0, 0.6, 2.4);
        let is_sphere = rand(i, 1) < 0.4;
        let entity = ec.create_entity();
        ec.set_components(
            entity,
            (
                LocalTransform {
                    position: Vec3::new(
                        rand_range(i, 2, -AREA_HALF, AREA_HALF),
                        scale * 0.5,
                        rand_range(i, 3, -AREA_HALF, AREA_HALF),
                    ),
                    rotation: Quat::rotation_axis_angle(random_axis(i, 4), rand_range(i, 7, 0.0, 6.28) as f64),
                    scale: Vec3::splat(scale),
                },
                MeshComponent {
                    mesh: if is_sphere { sphere.clone() } else { cube.clone() },
                },
                MaterialComponent {
                    material: materials[(rand(i, 8) * COLORS.len() as f32) as usize % COLORS.len()].clone(),
                },
            ),
        );
    }

    spawn_lights(&mut world);
}

fn lit(world: &mut WorldDataMut, color: Vec4<f32>, roughness: f32) -> AssetHandle<Material> {
    let mut data = MaterialData::new();
    data.insert("color", MaterialDataValue::Vec4(color));
    data.insert("metallic", MaterialDataValue::F32(0.0));
    data.insert("roughness", MaterialDataValue::F32(roughness));

    let material = create_material_from_template_key(world.resources(), BUILTIN_TEMPLATE_OPAQUE_LIT, data).unwrap();
    world
        .resources_mut()
        .get_mut::<AssetStorageResource<Material>>()
        .unwrap()
        .insert(material)
}

fn spawn_lights(world: &mut WorldDataMut) {
    let mut ec = world.entities_mut();

    for i in 1_000_000..1_000_000 + LIGHTS_COUNT {
        let hue = rand(i, 0);
        let color = hue_to_rgb(hue) * rand_range(i, 1, 10.0, 20.0);
        let range = rand_range(i, 2, 4.0, 10.0);
        let is_spot = rand(i, 3) < 0.3;
        let base = Vec3::new(
            rand_range(i, 4, -AREA_HALF, AREA_HALF),
            rand_range(i, 5, 0.5, 4.0),
            rand_range(i, 6, -AREA_HALF, AREA_HALF),
        );

        let light = if is_spot {
            StandardLightComponent::Spot {
                color,
                range,
                fov: rand_range(i, 7, 30_f32.to_radians(), 90_f32.to_radians()),
                shadows: false,
            }
        } else {
            StandardLightComponent::Point {
                color,
                range,
                shadows: false,
            }
        };

        let entity = ec.create_entity();
        ec.set_components(
            entity,
            (
                LocalTransform {
                    position: base,
                    // spots point along -Y, tilted a little
                    rotation: Quat::rotation_axis_angle(random_axis(i, 8), rand_range(i, 11, 0.0, 0.6) as f64),
                    scale: Vec3::splat(1.0),
                },
                light,
                LightMotion {
                    base,
                    amplitude: Vec3::new(rand_range(i, 12, 2.0, 8.0), 0.5, rand_range(i, 13, 2.0, 8.0)),
                    frequency: Vec3::new(
                        rand_range(i, 14, 0.1, 0.4),
                        rand_range(i, 15, 0.3, 0.8),
                        rand_range(i, 16, 0.1, 0.4),
                    ),
                    phase: Vec3::new(rand(i, 17) * 6.28, rand(i, 18) * 6.28, rand(i, 19) * 6.28),
                },
            ),
        );
    }
}

#[derive(Default)]
pub struct LightsClock(f32);

// Lissajous paths around each light's base
pub fn animate_lights(
    time: Res<TimeResource>,
    mut clock: Local<LightsClock>,
    mut motions: WorldQuery<(&mut LocalTransform, &LightMotion)>,
) {
    clock.0 += time.delta_time;
    let t = clock.0;

    for (transform, motion) in motions.iter_mut() {
        let wave = |axis: usize| (motion.frequency.as_array()[axis] * t + motion.phase.as_array()[axis]).sin();
        transform.position = motion.base
            + Vec3::new(
                motion.amplitude.x * wave(0),
                motion.amplitude.y * wave(1),
                motion.amplitude.z * wave(2),
            );
    }
}

fn hue_to_rgb(hue: f32) -> Vec3<f32> {
    let channel = |offset: f32| (((hue + offset).fract() * 6.0 - 3.0).abs() - 1.0).clamp(0.0, 1.0);
    Vec3::new(channel(0.0), channel(2.0 / 3.0), channel(1.0 / 3.0))
}
