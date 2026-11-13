//! Rigidbody physics, drawn as collider wireframes with gizmos. Four boxes fall onto a glass
//! platform; three bouncy spheres on a "ghost" layer fall through the glass and land on the floor;
//! a dumbbell made of three child colliders tumbles down; a kinematic paddle spins at the foot of
//! the ramp, partly sunk into its slope. Colors: gray floor, light blue glass, green dynamic
//! bodies, blue kinematic paddle.
//! Under the cursor, a red marker shows the raycast hit point and its normal; while a body is
//! dragged, a yellow line joins the grabbed point to the cursor target.
//!
//! Controls: left drag grabs and pulls a dynamic body; right drag orbits the camera, the scroll
//! wheel zooms, W/S/A/D move the camera pivot horizontally and R/F up and down; Space drops a new
//! spinning box.
//!
//! The floor is a triangle mesh collider. The ghost/glass pair is switched off in the physics
//! layer matrix, so those layers ignore each other while both still collide with the default
//! layer. The drag is a spring-damper force applied at the grabbed point in `FixedUpdate`, before
//! the physics step.
//!
//! Engine modules: `aphyn_physics` (`RigidbodyComponent`, `PhysicsMaterialComponent`,
//! `PhysicsResource` layer matrix, `PhysicsWorldResource` raycasts, forces), `aphyn_collision`
//! (`ColliderComponent`, shapes, `MeshCollider`, `CollisionLayer`), `aphyn_render_features`
//! (gizmos, `CameraComponent`), `aphyn_render` (`Camera`, `RenderSpace`), `aphyn_app` (input,
//! `TimeResource`), `aphyn_window` (`WindowResource`), `aphyn_transform` (parent/child),
//! `aphyn_ecs`.

use aphyn::*;

const LAYER_DEFAULT: CollisionLayer = CollisionLayer::DEFAULT;
const LAYER_GHOST: CollisionLayer = CollisionLayer::new(1).unwrap();
const LAYER_GLASS: CollisionLayer = CollisionLayer::new(2).unwrap();

const DRAG_STIFFNESS: f32 = 60.0;
const DRAG_DAMPING: f32 = 8.0;

aphyn_entry_point!(init_app);

pub fn init_app(ctx: AppInitCtx) {
    let AppInitCtx { mut behavior, .. } = ctx;

    let mut start = behavior.as_mut();
    start.get_mut(Schedule::Start).insert_system(setup_scene);

    let mut fixed = behavior.as_mut();
    let mut fixed = fixed.get_mut(Schedule::FixedUpdate);
    fixed.insert_system(apply_drag_force);
    fixed.order_system(apply_drag_force).before_group(SYSTEM_GROUP_PHYSICS);

    let mut update = behavior.as_mut();
    let mut update = update.get_mut(Schedule::Update);
    update.insert_system(orbit_camera);
    update.insert_system(spawn_on_space);
    update.insert_system(drag_bodies);
    update.insert_system(draw_colliders);
    update.order_system(orbit_camera).before_group(SYSTEM_GROUP_TRANSFORM);
    update.order_group(SYSTEM_GROUP_TRANSFORM).before_system(drag_bodies);
    update.order_system(drag_bodies).before_system(draw_colliders);
    update.order_system(draw_colliders).before_group(SYSTEM_GROUP_RENDER);
}

#[derive(Resource)]
struct CameraRig {
    pivot: Vec3<f32>,
    yaw: f32,
    pitch: f32,
    distance: f32,
    last_mouse: Option<[f64; 2]>,
}

struct Grab {
    body: EntityId,
    local_offset: Vec3<f32>,
    depth: f32,
    target: Vec3<f32>,
    last_point: Option<Vec3<f32>>,
}

#[derive(Resource, Default)]
struct DragState {
    grab: Option<Grab>,
    hover: Option<(Vec3<f32>, Vec3<f32>)>,
    rope: Option<(Vec3<f32>, Vec3<f32>)>,
}

fn sphere(center: Vec3<f32>, radius: f32) -> CollisionSphere {
    CollisionSphere { center, radius }
}

fn setup_scene(mut world: WorldDataMut) {
    {
        let mut res = world.resources_mut();
        let physics = res.get_mut::<PhysicsResource>().unwrap();
        // ghost spheres fall through the glass platform but still land on the floor
        physics.layer_matrix.set(LAYER_GHOST, LAYER_GLASS, false);
        res.insert(CameraRig {
            pivot: Vec3::new(0.0, 1.0, 0.0),
            yaw: 0.0,
            pitch: 0.35,
            distance: 16.0,
            last_mouse: None,
        });
        res.insert(DragState::default());
    }

    let mut ec = world.entities_mut();

    let camera = ec.create_entity();
    ec.set_components(
        camera,
        (
            LocalTransform::IDENTITY,
            CameraComponent(Camera {
                near: 0.1,
                far: 1_000.0,
                projection: CameraProjection::Perspective {
                    fov: 70_f32.to_radians(),
                },
            }),
        ),
    );

    // floor: a triangle grid with a ramp along +x
    let floor = ec.create_entity();
    ec.set_components(floor, (LocalTransform::IDENTITY, ColliderComponent::new(floor_mesh())));

    // glass platform, only the default layer collides with it
    let glass_box = CollisionBox {
        center: Vec3::new(-3.0, 3.0, 0.0),
        extents: Vec3::new(2.5, 0.1, 2.5),
        rotation: Quat::IDENTITY,
    };
    let glass = ec.create_entity();
    ec.set_components(
        glass,
        (
            LocalTransform::IDENTITY,
            ColliderComponent::new(glass_box).with_layer(LAYER_GLASS),
        ),
    );

    for i in 0..4 {
        let position = Vec3::new(-3.5 + 0.4 * i as f32, 5.0 + 1.3 * i as f32, 0.2 * i as f32);
        spawn_body(
            &mut ec,
            position,
            Quat::rotation_y(0.3 * i as f64),
            RigidbodyComponent::default(),
            ColliderComponent::new(CollisionBox::default()).with_layer(LAYER_DEFAULT),
        );
    }

    for i in 0..3 {
        let ghost = spawn_body(
            &mut ec,
            Vec3::new(-2.5 + 0.7 * i as f32, 8.0 + i as f32, -1.0),
            Quat::IDENTITY,
            RigidbodyComponent::default(),
            ColliderComponent::new(sphere(Vec3::splat(0.0), 0.4)).with_layer(LAYER_GHOST),
        );
        let bouncy = PhysicsMaterial {
            friction: 0.4,
            restitution: 0.6,
        };
        ec.add_component(ghost, PhysicsMaterialComponent(bouncy)).ok().unwrap();
    }

    // compound body: a dumbbell made of child colliders
    let dumbbell = ec.create_entity();
    let dumbbell_transform = LocalTransform {
        position: Vec3::new(3.0, 6.0, 0.0),
        rotation: Quat::rotation_z(0.5),
        scale: Vec3::splat(1.0),
    };
    ec.set_components(dumbbell, (dumbbell_transform, RigidbodyComponent::default()));
    let parts: [(Vec3<f32>, ColliderShape); 3] = [
        (Vec3::new(-1.2, 0.0, 0.0), sphere(Vec3::splat(0.0), 0.5).into()),
        (Vec3::new(1.2, 0.0, 0.0), sphere(Vec3::splat(0.0), 0.5).into()),
        (
            Vec3::splat(0.0),
            CollisionBox {
                extents: Vec3::new(1.0, 0.15, 0.15),
                ..Default::default()
            }
            .into(),
        ),
    ];
    for (offset, shape) in parts {
        let part = ec.create_entity();
        let transform = LocalTransform {
            position: offset,
            ..LocalTransform::IDENTITY
        };
        ec.set_components(
            part,
            (transform, ChildComponent { parent: dumbbell }, ColliderComponent::new(shape)),
        );
    }

    // kinematic spinning paddle
    let paddle = RigidbodyComponent {
        angular_velocity: Vec3::new(0.0, 1.5, 0.0),
        ..RigidbodyComponent::kinematic()
    };
    let paddle_box = CollisionBox {
        extents: Vec3::new(2.5, 0.3, 0.2),
        ..Default::default()
    };
    spawn_body(
        &mut ec,
        Vec3::new(5.0, 0.3, 4.0),
        Quat::IDENTITY,
        paddle,
        ColliderComponent::new(paddle_box),
    );
}

fn spawn_body(
    ec: &mut EntitiesHolderMut,
    position: Vec3<f32>,
    rotation: Quat<f32>,
    rigidbody: RigidbodyComponent,
    collider: ColliderComponent,
) -> EntityId {
    let body = ec.create_entity();
    let transform = LocalTransform {
        position,
        rotation,
        scale: Vec3::splat(1.0),
    };
    ec.set_components(body, (transform, rigidbody, collider));
    body
}

fn floor_mesh() -> MeshCollider {
    let height = |x: f32| if x > 4.0 { (x - 4.0) * 0.4 } else { 0.0 };
    let mut triangles = Vec::new();

    for i in -8..8 {
        for j in -8..8 {
            let (x0, z0) = (i as f32, j as f32);
            let (x1, z1) = (x0 + 1.0, z0 + 1.0);
            let a = Vec3::new(x0, height(x0), z0);
            let b = Vec3::new(x1, height(x1), z0);
            let c = Vec3::new(x1, height(x1), z1);
            let d = Vec3::new(x0, height(x0), z1);
            triangles.push([a, b, c]);
            triangles.push([a, c, d]);
        }
    }

    MeshCollider::new(triangles)
}

fn spawn_on_space(mut world: WorldDataMut) {
    let (pressed, frame) = {
        let res = world.resources();
        let pressed = res.get::<InputResource>().unwrap().keyboard.is_just_pressed(KeyCode::Space);
        (pressed, res.get::<TimeResource>().unwrap().frame_number)
    };
    if !pressed {
        return;
    }

    let jitter = (frame % 7) as f32 * 0.1;
    let rigidbody = RigidbodyComponent {
        angular_velocity: Vec3::new(1.0 + jitter, 2.0, 0.5),
        ..Default::default()
    };
    spawn_body(
        &mut world.entities_mut(),
        Vec3::new(jitter, 9.0, jitter),
        Quat::IDENTITY,
        rigidbody,
        ColliderComponent::new(CollisionBox::default()),
    );
}

fn draw_colliders(
    colliders: WorldQuery<(
        &ColliderComponent,
        &GlobalTransform,
        Option<&RigidbodyComponent>,
        Option<&ChildComponent>,
    )>,
    drag: Res<DragState>,
    mut gizmos: ResMut<GizmosResource>,
) {
    let lines = gizmos.space(RenderSpace::World);

    for (collider, transform, rigidbody, child) in colliders.iter() {
        let color = match (rigidbody, child) {
            (Some(rb), _) if rb.body_type == RigidbodyType::Kinematic => Vec4::new(0.3, 0.5, 1.0, 1.0),
            (Some(_), _) | (None, Some(_)) => Vec4::new(0.3, 1.0, 0.3, 1.0),
            (None, None) if collider.layer == LAYER_GLASS => Vec4::new(0.6, 0.9, 1.0, 1.0),
            (None, None) => Vec4::new(0.5, 0.5, 0.5, 1.0),
        };
        let matrix = transform.scale_rotation.into_4x4_with_offset(transform.position);

        match &collider.shape {
            ColliderShape::Primitive(shape) => draw_shape(lines, color, shape.apply_matrix_lossy(matrix)),
            ColliderShape::Mesh(mesh) => {
                for tri in mesh.triangles() {
                    draw_shape(
                        lines,
                        color,
                        CollisionShape::Triangle(tri.map(|p| matrix.mul_with_projection(p))),
                    );
                }
            }
        }
    }

    let red = Vec4::new(1.0, 0.2, 0.2, 1.0);
    if let Some((point, normal)) = drag.hover {
        lines.push(GizmoLine {
            start: point,
            end: point + normal,
            color: red,
        });
        draw_shape(lines, red, sphere(point, 0.1).into());
    }
    if let Some((grab, target)) = drag.rope {
        let yellow = Vec4::new(1.0, 0.9, 0.2, 1.0);
        lines.push(GizmoLine {
            start: grab,
            end: target,
            color: yellow,
        });
        draw_shape(lines, yellow, sphere(target, 0.08).into());
    }
}

fn draw_shape(lines: &mut FfiVec<GizmoLine>, color: Vec4<f32>, shape: CollisionShape) {
    let mut line = |start: Vec3<f32>, end: Vec3<f32>| lines.push(GizmoLine { start, end, color });

    match shape {
        CollisionShape::Triangle([a, b, c]) => {
            line(a, b);
            line(b, c);
            line(c, a);
        }
        CollisionShape::Box(CollisionBox {
            center,
            extents,
            rotation,
        }) => draw_box(&mut line, center, extents, rotation.to_matrix()),
        CollisionShape::Aabb(CollisionAabb { center, extents }) => draw_box(&mut line, center, extents, Mat::IDENTITY),
        CollisionShape::Sphere(CollisionSphere { center, radius }) => {
            const SEGMENTS: usize = 16;
            // one circle in each axis plane
            for axis in 0..3 {
                let point = |i: usize| {
                    let angle = i as f32 / SEGMENTS as f32 * std::f32::consts::TAU;
                    let (s, c) = angle.sin_cos();
                    let mut offset = [0.0; 3];
                    offset[(axis + 1) % 3] = c * radius;
                    offset[(axis + 2) % 3] = s * radius;
                    center + Vec3::from_array(offset)
                };
                for i in 0..SEGMENTS {
                    line(point(i), point(i + 1));
                }
            }
        }
        CollisionShape::Line(l) => line(l.start, l.end),
        CollisionShape::Point(_) => {}
    }
}

fn draw_box(line: &mut impl FnMut(Vec3<f32>, Vec3<f32>), center: Vec3<f32>, extents: Vec3<f32>, rotation: Mat3<f32>) {
    // corner i is on the +x, +y, +z side when bit 1, 2, 4 of i is set
    let local = CollisionAabb {
        center: Vec3::splat(0.0),
        extents,
    }
    .corners();
    let corner = |i: usize| center + rotation * local[i];
    // an edge joins two corners that differ in exactly one bit
    for a in 0..8 {
        for bit in [1, 2, 4] {
            if a & bit == 0 {
                line(corner(a), corner(a | bit));
            }
        }
    }
}

// right drag orbits the pivot, scroll zooms, WASD/RF move the pivot
fn orbit_camera(
    mut q: WorldQuery<&mut LocalTransform, WithFilter<CameraComponent>>,
    mut rig: ResMut<CameraRig>,
    input: Res<InputResource>,
    time: Res<TimeResource>,
) {
    let mouse = input.mouse.position;
    if let Some(last) = rig.last_mouse
        && input.mouse.is_pressed(MouseButton::Right)
    {
        rig.yaw += (mouse[0] - last[0]) as f32 * 0.006;
        rig.pitch = (rig.pitch + (mouse[1] - last[1]) as f32 * 0.006).clamp(-1.5, 1.5);
    }
    rig.last_mouse = Some(mouse);

    rig.distance = (rig.distance * 0.9_f32.powf(input.mouse.scroll_delta_lines[1])).clamp(2.0, 60.0);

    let mut direction = Vec3::<f32>::splat(0.0);
    // (key for +axis, key for -axis, axis)
    let keys: [(KeyCode, KeyCode, Vec3<f32>); 3] = [
        (KeyCode::KeyD, KeyCode::KeyA, Vec3::X),
        (KeyCode::KeyR, KeyCode::KeyF, Vec3::Y),
        (KeyCode::KeyW, KeyCode::KeyS, Vec3::Z),
    ];
    for (plus, minus, axis) in keys {
        if input.keyboard.is_pressed(plus) {
            direction += axis;
        }
        if input.keyboard.is_pressed(minus) {
            direction -= axis;
        }
    }
    let yaw = Quat::rotation_y(rig.yaw as f64);
    rig.pivot += yaw.to_matrix() * direction * (6.0 * time.delta_time);

    let rotation = yaw * Quat::rotation_x(rig.pitch as f64);
    let position = rig.pivot - rotation.to_matrix() * Vec3::Z * rig.distance;
    for transform in q.iter_mut() {
        transform.position = position;
        transform.rotation = rotation;
    }
}

// camera-local view space looks down +z; fov is the vertical field of view
fn mouse_ray(camera: &GlobalTransform, projection: CameraProjection, mouse: [f64; 2], size: [u32; 2]) -> Option<Vec3<f32>> {
    let CameraProjection::Perspective { fov } = projection else {
        return None;
    };
    if size[0] == 0 || size[1] == 0 {
        return None;
    }
    let tan = (fov * 0.5).tan();
    let aspect = size[0] as f32 / size[1] as f32;
    let x = (2.0 * mouse[0] / size[0] as f64 - 1.0) as f32;
    let y = (1.0 - 2.0 * mouse[1] / size[1] as f64) as f32;
    Some((camera.scale_rotation * Vec3::new(x * tan * aspect, y * tan, 1.0)).normalized())
}

// left press grabs a dynamic body; the cursor target lives in the camera-facing plane through the press point
fn drag_bodies(
    cameras: WorldQuery<(&GlobalTransform, &CameraComponent)>,
    bodies: WorldQuery<(&RigidbodyComponent, &GlobalTransform)>,
    input: Res<InputResource>,
    window: Res<WindowResource>,
    physics: Res<PhysicsWorldResource>,
    mut drag: ResMut<DragState>,
) {
    drag.hover = None;
    drag.rope = None;

    let Some((camera, camera_component)) = cameras.iter().next() else {
        return;
    };
    let Some(ray) = mouse_ray(
        camera,
        camera_component.0.projection,
        input.mouse.position,
        window.state().size,
    ) else {
        return;
    };
    let origin = camera.position;
    let forward = (camera.scale_rotation * Vec3::Z).normalized();

    let line = CollisionLine {
        start: origin,
        end: origin + ray * 200.0,
        bounds: LineBoundType::SEGMENT,
    };
    let hit = physics.raycast(line, COLLISION_MASK_ALL);
    drag.hover = hit.as_ref().map(|hit| (hit.point, hit.normal));

    if input.mouse.is_just_released(MouseButton::Left) {
        drag.grab = None;
    }

    if input.mouse.is_just_pressed(MouseButton::Left)
        && let Some(hit) = hit
        && let Some(body) = hit.body
        && let Some((rigidbody, transform)) = bodies.get(body)
        && rigidbody.body_type == RigidbodyType::Dynamic
        && let Some(inverse) = transform.scale_rotation.inverse()
    {
        drag.grab = Some(Grab {
            body,
            local_offset: inverse * (hit.point - transform.position),
            depth: (hit.point - origin).dot(forward),
            target: hit.point,
            last_point: None,
        });
    }

    let Some(grab) = &mut drag.grab else {
        return;
    };
    let Some((_, transform)) = bodies.get(grab.body) else {
        drag.grab = None;
        return;
    };

    grab.target = origin + ray * (grab.depth / ray.dot(forward));
    let grab_point = transform.position + transform.scale_rotation * grab.local_offset;
    drag.rope = Some((grab_point, grab.target));
}

// spring-damper force at the grabbed point, applied every fixed step before physics
fn apply_drag_force(
    mut bodies: WorldQuery<(&mut RigidbodyComponent, &GlobalTransform)>,
    time: Res<TimeResource>,
    mut drag: ResMut<DragState>,
) {
    let Some(grab) = &mut drag.grab else {
        return;
    };
    let Some((rigidbody, transform)) = bodies.get_mut(grab.body) else {
        return;
    };

    let point = transform.position + transform.scale_rotation * grab.local_offset;
    let point_velocity = match grab.last_point {
        Some(last) if time.fixed_delta_time > 0.0 => (point - last) * (1.0 / time.fixed_delta_time),
        _ => Vec3::splat(0.0),
    };
    grab.last_point = Some(point);

    let force = ((grab.target - point) * DRAG_STIFFNESS - point_velocity * DRAG_DAMPING) * rigidbody.mass;
    rigidbody.add_force_at_point(force, point);
}
