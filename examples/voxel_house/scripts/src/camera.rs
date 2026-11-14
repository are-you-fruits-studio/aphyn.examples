use aphyn::*;

// distance from the vertical line through the house center
const ORBIT_RADIUS: f32 = 13.0;
const ORBIT_HEIGHT: f32 = 5.5;
// radians per second, one lap in about 18 s
const ORBIT_SPEED: f32 = 0.35;
// step cap, so a long frame such as a rebuild does not make the camera jump;
// below 20 fps the orbit runs in slow motion
const MAX_STEP: f32 = 0.05;
const TARGET: Vec3<f32> = Vec3::new(0.0, 1.8, 0.0);

#[derive(Component)]
pub struct CameraOrbit {
    // 0 puts the camera on the -Z side of the house, looking towards +Z
    pub angle: f32,
}

pub fn orbit_camera(time: Res<TimeResource>, mut cameras: WorldQuery<(&mut LocalTransform, &mut CameraOrbit)>) {
    let dt = time.delta_time.min(MAX_STEP);

    for (transform, orbit) in cameras.iter_mut() {
        orbit.angle += ORBIT_SPEED * dt;
        *transform = orbit_transform(orbit.angle);
    }
}

pub fn orbit_transform(angle: f32) -> LocalTransform {
    let position = Vec3::new(ORBIT_RADIUS * angle.sin(), ORBIT_HEIGHT, -ORBIT_RADIUS * angle.cos());
    LocalTransform {
        position,
        rotation: Quat::look_rotation(TARGET - position, Vec3::new(0.0, 1.0, 0.0)),
        scale: Vec3::splat(1.0),
    }
}
