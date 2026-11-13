//! A hundred triangles that flock: each boid is pulled towards the others when they are far and
//! pushed away when they are close, steers through a motor with damping, turns to face where it
//! is going and stays inside a box.
//!
//! Shows a simulation split into small systems that run in a fixed order
//! (`order_system(..).before_system(..)`), shared tuning values in a resource, and entities that
//! are placed through their `LocalTransform` (position and a `Quat::rotation_z` heading), from
//! which the transform module derives the `GlobalTransform` that rendering reads.
//!
//! The simulation runs in `Schedule::FixedUpdate` (50 steps per second by default) instead of
//! `Update`: its per-step constants then give the same flock speed at any frame rate.
//!
//! Engine modules: `aphyn_ecs` (components, resources, ordered systems, `FixedUpdate`),
//! `aphyn_transform` (`LocalTransform`), `aphyn_math` (`Vec3`, `Quat`), `aphyn_render`
//! (`RenderApiResource`, mesh built in code, `Camera`), `aphyn_render_features` (unlit material
//! template, `MeshComponent`, `CameraComponent`).

use aphyn::*;

const BOID_COUNT: usize = 100;
const BOUNDS: f32 = 5.0;

aphyn_entry_point!(init_app);

pub fn init_app(ctx: AppInitCtx) {
    let AppInitCtx {
        mut data, mut behavior, ..
    } = ctx;

    data.as_mut().resources_mut().insert(BoidSettings {
        attraction_threshold: 1.0,
        damping_factor: 0.2,
    });

    let mut start = behavior.as_mut();
    let mut start = start.get_mut(Schedule::Start);
    start.insert_system(spawn_boids);
    start.order_group(SYSTEM_GROUP_RENDER).before_system(spawn_boids);

    let mut fixed = behavior.as_mut();
    let mut fixed = fixed.get_mut(Schedule::FixedUpdate);
    fixed.insert_system(accumulate_boid_separation);
    fixed.insert_system(affect_motor_by_boid);
    fixed.insert_system(apply_motor);
    fixed.insert_system(apply_damping);
    fixed.insert_system(apply_velocity);
    fixed.insert_system(rotate_boids_by_velocity);
    fixed.insert_system(restrict_boids);

    fixed
        .order_system(accumulate_boid_separation)
        .before_system(affect_motor_by_boid);
    fixed.order_system(affect_motor_by_boid).before_system(apply_motor);
    fixed.order_system(apply_motor).before_system(apply_damping);
    fixed.order_system(apply_damping).before_system(apply_velocity);
    fixed.order_system(apply_velocity).before_system(rotate_boids_by_velocity);
    fixed.order_system(rotate_boids_by_velocity).before_system(restrict_boids);
}

#[derive(Component)]
struct Boid {
    target_direction: Vec3<f32>,
}

#[derive(Component)]
struct Velocity(Vec3<f32>);

#[derive(Component)]
struct Motor {
    acceleration_direction: Vec3<f32>,
    strength: f32,
}

// Anything boids flock around; here every boid is also a target for the others.
#[derive(Component)]
struct BoidTarget;

#[derive(Resource)]
struct BoidSettings {
    // Below this distance a target pushes a boid away, above it pulls it closer.
    attraction_threshold: f32,
    damping_factor: f32,
}

fn spawn_boids(mut world: WorldDataMut) {
    let render_api = world.as_ref().resources().get::<RenderApiResource>().unwrap();

    let material =
        create_material_from_template_key(world.as_ref().resources(), BUILTIN_TEMPLATE_OPAQUE_UNLIT, MaterialData::new()).unwrap();

    // a triangle pointing up +Y, dark at the base and blue at the tip
    let mut vertices = [
        StandardVertex {
            position: [0.0, 0.0, 0.0],
            color: [0.0, 0.0, 0.0, 1.0],
            ..Default::default()
        },
        StandardVertex {
            position: [0.5, 1.0, 0.0],
            color: [0.2, 0.5, 1.0, 1.0],
            ..Default::default()
        },
        StandardVertex {
            position: [1.0, 0.0, 0.0],
            color: [0.0, 0.0, 0.0, 1.0],
            ..Default::default()
        },
    ];

    for vertex in vertices.iter_mut() {
        vertex.position[0] -= 0.5;
        vertex.position[1] -= 0.5;

        for coordinate in vertex.position.iter_mut() {
            *coordinate *= 0.2;
        }
    }

    let mesh = render_api.create_mesh(&vertices, &[0, 2, 1], None);

    let material = world
        .as_mut()
        .resources_mut()
        .get_mut::<AssetStorageResource<Material>>()
        .unwrap()
        .insert(material);
    let mesh = world
        .as_mut()
        .resources_mut()
        .get_mut::<AssetStorageResource<StandardMesh>>()
        .unwrap()
        .insert(mesh);

    let mut ec = world.entities_mut();

    for _ in 0..BOID_COUNT {
        let boid = ec.create_entity();
        ec.set_components(
            boid,
            (
                MeshComponent { mesh: mesh.clone() },
                MaterialComponent {
                    material: material.clone(),
                },
                Boid {
                    target_direction: Vec3::splat(0.0),
                },
                BoidTarget,
                Motor {
                    acceleration_direction: Vec3::splat(0.0),
                    strength: 0.01,
                },
                Velocity(Vec3::splat(0.0)),
                LocalTransform {
                    position: Vec3::new(rand::random::<f32>(), rand::random::<f32>(), 0.0),
                    ..LocalTransform::IDENTITY
                },
            ),
        );
    }

    let camera = ec.create_entity();
    ec.set_components(
        camera,
        (
            LocalTransform {
                position: Vec3::new(0.0, 0.0, -5.0),
                ..LocalTransform::IDENTITY
            },
            CameraComponent(Camera {
                near: 0.1,
                far: 1_000.0,
                projection: CameraProjection::Perspective {
                    fov: 90_f32.to_radians(),
                },
            }),
        ),
    );
}

fn accumulate_boid_separation(
    settings: Res<BoidSettings>,
    targets: WorldQuery<(EntityId, &LocalTransform, &BoidTarget)>,
    mut boids: WorldQuery<(EntityId, &LocalTransform, &mut Boid)>,
) {
    for (boid_entity, boid_transform, boid) in boids.iter_mut() {
        let mut sum = Vec3::splat(0.0);

        for (target_entity, target_transform, _) in targets.iter() {
            if target_entity == boid_entity {
                continue;
            }

            let difference = target_transform.position - boid_transform.position;
            let attraction = difference.length() as f32 - settings.attraction_threshold;

            sum += difference.normalized() * attraction;
        }

        boid.target_direction += sum.normalized();
    }
}

fn affect_motor_by_boid(mut query: WorldQuery<(&Boid, &mut Motor)>) {
    for (boid, motor) in query.iter_mut() {
        motor.acceleration_direction = (motor.acceleration_direction.normalized() + boid.target_direction).normalized();
    }
}

fn apply_motor(mut query: WorldQuery<(&Motor, &mut Velocity)>) {
    for (motor, velocity) in query.iter_mut() {
        velocity.0 += motor.acceleration_direction.normalized() * motor.strength;
    }
}

fn apply_damping(settings: Res<BoidSettings>, mut query: WorldQuery<&mut Velocity>) {
    for velocity in query.iter_mut() {
        velocity.0 -= velocity.0 * settings.damping_factor;
    }
}

fn apply_velocity(mut query: WorldQuery<(&Velocity, &mut LocalTransform)>) {
    for (velocity, transform) in query.iter_mut() {
        transform.position += velocity.0;
    }
}

fn rotate_boids_by_velocity(mut query: WorldQuery<(&mut LocalTransform, &Velocity, &Boid)>) {
    for (transform, velocity, _) in query.iter_mut() {
        let angle = f32::atan2(velocity.0.x, velocity.0.y);
        transform.rotation = Quat::rotation_z(-angle as f64);
    }
}

fn restrict_boids(mut query: WorldQuery<(&mut LocalTransform, &Boid)>) {
    for (transform, _) in query.iter_mut() {
        transform.position = transform.position.map(|c| c.clamp(-BOUNDS, BOUNDS));
    }
}
