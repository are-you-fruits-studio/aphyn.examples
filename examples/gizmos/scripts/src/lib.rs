//! Debug lines in the three gizmo spaces: a red square in clip space (always centered, follows
//! the window), a green square in window pixels (fixed in the top-left corner) and a blue cube in
//! world space that you can fly around.
//!
//! Controls: W/S forward and back, A/D left and right, R/F up and down, Q/E turn.
//!
//! Gizmos are immediate mode: the lines are pushed into `GizmosResource` every frame and drawn
//! once. They have no depth test, so they always draw on top of meshes.
//!
//! Engine modules: `aphyn_render_features` (`GizmosResource`, `CameraComponent`), `aphyn_render`
//! (`RenderSpace`, `Camera`), `aphyn_transform` (camera transform), `aphyn_app` (`InputResource`,
//! `TimeResource`), `aphyn_ecs`.

use aphyn::*;

// units and radians per second
const CAMERA_SPEED: f32 = 0.6;
const CAMERA_TURN_SPEED: f64 = 0.6;

aphyn_entry_point!(init_app);

pub fn init_app(ctx: AppInitCtx) {
    let AppInitCtx {
        mut data, mut behavior, ..
    } = ctx;

    let mut update = behavior.as_mut();
    let mut update = update.get_mut(Schedule::Update);
    update.insert_system(move_camera);
    update.insert_system(draw_gizmos);

    update.order_system(move_camera).before_group(SYSTEM_GROUP_TRANSFORM);
    update.order_system(draw_gizmos).before_group(SYSTEM_GROUP_RENDER);

    let mut ec = data.as_mut().into_entities_mut();
    let camera = ec.create_entity();
    ec.set_components(
        camera,
        (
            LocalTransform::IDENTITY,
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

fn move_camera(
    input: Res<InputResource>,
    time: Res<TimeResource>,
    mut cameras: WorldQuery<(&mut LocalTransform, &CameraComponent)>,
) {
    let key = |code| if input.keyboard.is_pressed(code) { 1.0 } else { 0.0 };

    let direction = Vec3::new(
        key(KeyCode::KeyD) - key(KeyCode::KeyA),
        key(KeyCode::KeyR) - key(KeyCode::KeyF),
        key(KeyCode::KeyW) - key(KeyCode::KeyS),
    );
    let turn = f64::from(key(KeyCode::KeyE) - key(KeyCode::KeyQ));
    let dt = time.delta_time;

    for (transform, _) in cameras.iter_mut() {
        transform.position += transform.rotation.to_matrix() * direction * (CAMERA_SPEED * dt);
        transform.rotation = Quat::rotation_y(turn * CAMERA_TURN_SPEED * f64::from(dt)) * transform.rotation;
    }
}

fn draw_gizmos(mut gizmos: ResMut<GizmosResource>) {
    // clip space: -1..1 on both axes, whatever the window size
    draw_loop(
        gizmos.space(RenderSpace::Clip),
        Vec4::new(1.0, 0.0, 0.0, 1.0),
        &[
            Vec3::new(-0.5, -0.5, 0.0),
            Vec3::new(-0.5, 0.5, 0.0),
            Vec3::new(0.5, 0.5, 0.0),
            Vec3::new(0.5, -0.5, 0.0),
        ],
    );

    // window space: pixels from the top-left corner
    draw_loop(
        gizmos.space(RenderSpace::Window),
        Vec4::new(0.0, 1.0, 0.0, 1.0),
        &[
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(1.0, 100.0, 0.0),
            Vec3::new(100.0, 100.0, 0.0),
            Vec3::new(100.0, 1.0, 0.0),
        ],
    );

    // world space: a unit cube seen through the camera
    let lines = gizmos.space(RenderSpace::World);
    let color = Vec4::new(0.0, 0.0, 1.0, 1.0);
    let square = |z| {
        [
            Vec3::new(0.0, 0.0, z),
            Vec3::new(0.0, 1.0, z),
            Vec3::new(1.0, 1.0, z),
            Vec3::new(1.0, 0.0, z),
        ]
    };
    let (near, far) = (square(0.0), square(1.0));

    draw_loop(lines, color, &near);
    draw_loop(lines, color, &far);
    for (start, end) in near.into_iter().zip(far) {
        lines.push(GizmoLine { start, end, color });
    }
}

// Closed polyline: each point is joined to the next and the last to the first.
fn draw_loop(lines: &mut FfiVec<GizmoLine>, color: Vec4<f32>, points: &[Vec3<f32>]) {
    for (i, &start) in points.iter().enumerate() {
        let end = points[(i + 1) % points.len()];
        lines.push(GizmoLine { start, end, color });
    }
}
