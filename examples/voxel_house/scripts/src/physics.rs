use aphyn::*;

const GRAVITY: f32 = 18.0;
// the middle of the house, a little above the floor
const BLAST_CENTER: Vec3<f32> = Vec3::new(0.0, 1.0, 0.0);
// step cap, so a long frame such as a rebuild does not make the cubes jump;
// below 20 fps the explosion runs in slow motion
const MAX_STEP: f32 = 0.05;

#[derive(Resource, Default)]
pub struct Sim {
    pub exploded: bool,
}

#[derive(Component)]
pub struct Voxel {
    home: Vec3<f32>,
    seed: u32,
    velocity: Vec3<f32>,
    spin_axis: Vec3<f32>,
    spin: f32,
    size: f32,
}

impl Voxel {
    pub fn new(home: Vec3<f32>, seed: u32, size: f32) -> Self {
        Self {
            home,
            seed,
            velocity: Vec3::splat(0.0),
            spin_axis: Vec3::new(0.0, 1.0, 0.0),
            spin: 0.0,
            size,
        }
    }
}

// cheap deterministic random number in [0, 1) from a seed and a stream index
fn rand(seed: u32, n: u32) -> f32 {
    let mut h = seed
        .wrapping_mul(0x9E37_79B1)
        .wrapping_add(n.wrapping_mul(0x85EB_CA6B))
        .wrapping_add(0x1234_5678);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    h ^= h >> 12;
    h = h.wrapping_mul(0x297A_2D39);
    h ^= h >> 15;
    (h >> 8) as f32 / (1u32 << 24) as f32
}

// normalized lerp from `a` to `b` the short way around
fn nlerp(a: Quat<f32>, b: Quat<f32>, t: f32) -> Quat<f32> {
    let a = Vec4::from_array(a.into_array());
    let b = Vec4::from_array(b.into_array());
    let b = if a.dot(b) < 0.0 { -b } else { b };
    Quat::from_array(a.lerp(b, t).normalized().into_array())
}

// half the cube's vertical extent in its current orientation: how deep its lowest corner reaches
fn half_height(rotation: Quat<f32>, size: f32) -> f32 {
    let m = rotation.to_matrix();
    let m = m.as_array();
    0.5 * size * (m[0][1].abs() + m[1][1].abs() + m[2][1].abs())
}

fn explode(voxel: &mut Voxel, position: Vec3<f32>) {
    let d = position - BLAST_CENTER;
    let dist = d.length() as f32;
    let dir = if dist > 1e-3 {
        d * (1.0 / dist)
    } else {
        Vec3::new(0.0, 1.0, 0.0)
    };
    let seed = voxel.seed;
    // cubes near the center get kicked harder
    let power = (16.0 / (1.0 + 0.2 * dist)) * (0.7 + 0.6 * rand(seed, 0));
    let jitter = Vec3::new(rand(seed, 1) - 0.5, 0.0, rand(seed, 2) - 0.5) * 4.0;
    voxel.velocity = dir * power + jitter + Vec3::new(0.0, 5.0 + 6.0 * rand(seed, 3), 0.0);
    voxel.spin_axis = Vec3::new(rand(seed, 4) - 0.5, rand(seed, 5) - 0.5, rand(seed, 6) - 0.5);
    voxel.spin = 4.0 + 12.0 * rand(seed, 7);
}

pub fn simulate_voxels(
    time: Res<TimeResource>,
    input: Res<InputResource>,
    mut sim: ResMut<Sim>,
    mut voxels: WorldQuery<(&mut LocalTransform, &mut Voxel)>,
) {
    let dt = time.delta_time.min(MAX_STEP);

    let keyboard = &input.keyboard;
    let reset = keyboard.is_just_pressed(KeyCode::Tab) || keyboard.is_just_pressed(KeyCode::KeyR);
    let blast = keyboard.is_just_pressed(KeyCode::Space)
        || keyboard.is_just_pressed(KeyCode::Enter)
        || keyboard.is_just_pressed(KeyCode::NumpadEnter);

    if reset {
        sim.exploded = false;
        for (transform, voxel) in voxels.iter_mut() {
            transform.position = voxel.home;
            transform.rotation = Quat::IDENTITY;
            voxel.velocity = Vec3::splat(0.0);
            voxel.spin = 0.0;
        }
        return;
    }
    if blast && !sim.exploded {
        sim.exploded = true;
        for (transform, voxel) in voxels.iter_mut() {
            explode(voxel, transform.position);
        }
    }
    if !sim.exploded || dt == 0.0 {
        return;
    }

    for (transform, voxel) in voxels.iter_mut() {
        voxel.velocity.y -= GRAVITY * dt;
        transform.position += voxel.velocity * dt;
        if voxel.spin > 0.0 {
            transform.rotation =
                (Quat::rotation_axis_angle(voxel.spin_axis, (voxel.spin * dt) as f64) * transform.rotation).normalized();
        }

        let mut floor = half_height(transform.rotation, voxel.size);
        if transform.position.y <= floor + 0.01 {
            // on the ground: damp, and lay the cube flat once it has mostly stopped
            let slow = voxel.velocity.x * voxel.velocity.x + voxel.velocity.z * voxel.velocity.z < 4.0;
            if slow {
                transform.rotation = nlerp(transform.rotation, Quat::IDENTITY, (10.0 * dt).min(1.0));
                floor = half_height(transform.rotation, voxel.size);
            }
            transform.position.y = floor;
            voxel.velocity.y = if voxel.velocity.y < -1.0 {
                -voxel.velocity.y * 0.3
            } else {
                0.0
            };
            let friction = (1.0 - 5.0 * dt).max(0.0);
            voxel.velocity.x *= friction;
            voxel.velocity.z *= friction;
            voxel.spin *= (1.0 - 6.0 * dt).max(0.0);
            if voxel.spin < 0.05 {
                voxel.spin = 0.0;
            }
        }
    }
}
