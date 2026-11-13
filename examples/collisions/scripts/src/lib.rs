//! Collision shapes and queries, drawn with gizmos. A grid of 10 000 small squares in window
//! space turns red under the mouse cursor (green while the left button is held): each square has a
//! collider, and the cursor is a line raycast into the collision world, which returns the closest
//! hit. Two shapes in world space, an axis-aligned box and a rotated box, are tested against each
//! other with `overlaps`: they sit just apart, so they are drawn green, and would turn red if they
//! intersected. The FPS counter module prints the frame rate to the console once a second.
//!
//! Controls: mouse over the grid; W/S forward and back, A/D left and right, R/F up and down, Q/E
//! turn (moves the camera, so the world-space shapes move and the grid does not).
//!
//! The grid lives in window pixels: its colliders use pixel coordinates and the cursor position
//! is used directly, without the camera. Each collider is set in the cell's local space and the
//! collision world moves it by the cell's `GlobalTransform` (an AABB is translated, not scaled, so
//! the cells keep a unit scale and the collider carries the size). The outlines are drawn with the
//! same transform. Picking through a perspective camera needs a ray built from the camera instead.
//!
//! Engine modules: `aphyn_collision` (`ColliderComponent`, `CollisionWorldResource`, shapes,
//! `raycast`, `overlaps`), `aphyn_render_features` (gizmos, `CameraComponent`), `aphyn_render`
//! (`RenderSpace`, `Camera`), `aphyn_modules` (FPS counter), `aphyn_app` (input, `TimeResource`),
//! `aphyn_transform`, `aphyn_ecs`.

use std::f32::consts::TAU;

use aphyn::*;

const GRID_SIZE: usize = 100;
const GRID_STEP: f32 = 15.0;
const CELL_SIZE: f32 = 9.0;
// units and radians per second
const CAMERA_SPEED: f32 = 0.6;
const CAMERA_TURN_SPEED: f64 = 0.6;

const IDLE: Vec4<f32> = Vec4::new(0.5, 0.5, 0.5, 1.0);
const HOVERED: Vec4<f32> = Vec4::new(1.0, 0.0, 0.0, 1.0);
const PRESSED: Vec4<f32> = Vec4::new(0.0, 1.0, 0.0, 1.0);
const OVERLAPPING: Vec4<f32> = Vec4::new(1.0, 0.0, 0.0, 1.0);
const SEPARATE: Vec4<f32> = Vec4::new(0.0, 1.0, 0.0, 1.0);

aphyn_entry_point!(init_app);

pub fn init_app(ctx: AppInitCtx) {
    let AppInitCtx {
        mut data,
        mut behavior,
        render,
        ..
    } = ctx;

    fps_counter::add_module_to(data.as_mut(), behavior.as_mut(), render);

    let mut start = behavior.as_mut();
    start.get_mut(Schedule::Start).insert_system(spawn_scene);

    let mut update = behavior.as_mut();
    let mut update = update.get_mut(Schedule::Update);
    update.insert_system(move_camera);
    update.insert_system(draw_overlap_test);
    update.insert_system(raycast_mouse);
    update.insert_system(draw_cells);

    update.order_system(move_camera).before_group(SYSTEM_GROUP_TRANSFORM);
    update.order_system(raycast_mouse).before_system(draw_cells);
    update.order_system(draw_cells).before_group(SYSTEM_GROUP_RENDER);
    update.order_system(draw_overlap_test).before_group(SYSTEM_GROUP_RENDER);
}

// A grid cell: drawn as its collider's outline in the given gizmo space.
#[derive(Component)]
struct Cell {
    space: RenderSpace,
    color: Vec4<f32>,
}

fn spawn_scene(mut world: WorldDataMut) {
    let mut ec = world.entities_mut();

    // behind the overlap test, which sits around the world origin
    let camera = ec.create_entity();
    ec.set_components(
        camera,
        (
            LocalTransform {
                position: Vec3::new(1.6, 0.0, -3.0),
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

    for x in 0..GRID_SIZE {
        for y in 0..GRID_SIZE {
            let cell = ec.create_entity();
            ec.set_components(
                cell,
                (
                    LocalTransform {
                        position: Vec3::new(GRID_STEP * x as f32, GRID_STEP * y as f32, 0.0),
                        ..LocalTransform::IDENTITY
                    },
                    Cell {
                        space: RenderSpace::Window,
                        color: IDLE,
                    },
                    // in the cell's local space; the collision world moves it by the cell's GlobalTransform
                    ColliderComponent::new(CollisionAabb {
                        center: Vec3::splat(0.0),
                        extents: Vec3::new(CELL_SIZE / 2.0, CELL_SIZE / 2.0, 0.05),
                    }),
                ),
            );
        }
    }
}

fn raycast_mouse(collision_world: Res<CollisionWorldResource>, input: Res<InputResource>, mut cells: WorldQuery<&mut Cell>) {
    for cell in cells.iter_mut() {
        cell.color = IDLE;
    }

    // a line through the cursor, perpendicular to the window
    let start = Vec3::new(input.mouse.position[0] as f32, input.mouse.position[1] as f32, 0.0);
    let line = CollisionLine {
        start,
        end: start + Vec3::<f32>::Z,
        bounds: LineBoundType::UNRESTRICTED,
    };

    let Some((_, entity)) = collision_world.raycast(line, COLLISION_MASK_ALL) else {
        return;
    };
    if let Some(cell) = cells.get_mut(entity) {
        cell.color = if input.mouse.is_pressed(MouseButton::Left) {
            PRESSED
        } else {
            HOVERED
        };
    }
}

fn draw_cells(cells: WorldQuery<(&Cell, &ColliderComponent, &GlobalTransform)>, mut gizmos: ResMut<GizmosResource>) {
    for (cell, collider, transform) in cells.iter() {
        let ColliderShape::Primitive(shape) = collider.shape else {
            continue;
        };

        // the same transform the collision world applies, so the outline matches what the cursor hits
        let to_world = transform.scale_rotation.into_4x4_with_offset(transform.position);
        draw_shape(gizmos.space(cell.space), cell.color, shape.apply_matrix_lossy(to_world));
    }
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

// Two shapes tested directly with `overlaps`, without the collision world.
fn draw_overlap_test(mut gizmos: ResMut<GizmosResource>) {
    let aabb = CollisionAabb {
        center: Vec3::new(2.2, 0.0, 0.0),
        extents: Vec3::splat(0.5),
    }
    .into();
    let rotated_box = CollisionBox {
        center: Vec3::new(1.0, 0.0, 0.0),
        extents: Vec3::splat(0.5),
        rotation: Quat::rotation_y(1.0),
    }
    .into();

    let color = if overlaps(aabb, rotated_box) { OVERLAPPING } else { SEPARATE };

    let lines = gizmos.space(RenderSpace::World);
    draw_shape(lines, color, aabb);
    draw_shape(lines, color, rotated_box);
}

fn draw_shape(lines: &mut FfiVec<GizmoLine>, color: Vec4<f32>, shape: CollisionShape) {
    match shape {
        CollisionShape::Point(point) => draw_point(lines, color, point),
        CollisionShape::Line(line) => draw_line(lines, color, line),
        CollisionShape::Sphere(sphere) => draw_sphere(lines, color, sphere),
        CollisionShape::Aabb(aabb) => draw_box(lines, color, aabb.corners()),
        CollisionShape::Box(rotated) => {
            // the corners of the unrotated box around the origin, then rotated and moved to the center
            let local = CollisionAabb {
                center: Vec3::splat(0.0),
                extents: rotated.extents,
            };
            let rotation = rotated.rotation.to_matrix();
            draw_box(lines, color, local.corners().map(|corner| rotated.center + rotation * corner));
        }
        CollisionShape::Triangle(triangle) => draw_loop(lines, color, &triangle),
    }
}

fn draw_point(lines: &mut FfiVec<GizmoLine>, color: Vec4<f32>, point: Vec3<f32>) {
    let size = 0.05_f32;

    for axis in [Vec3::<f32>::X, Vec3::<f32>::Y, Vec3::<f32>::Z] {
        lines.push(GizmoLine {
            start: point - axis * size,
            end: point + axis * size,
            color,
        });
    }
}

fn draw_line(lines: &mut FfiVec<GizmoLine>, color: Vec4<f32>, line: CollisionLine) {
    // unrestricted ends are drawn long instead of infinite
    let mut start = line.start;
    let mut end = line.end;

    if !line.bounds.is_end_restricted() {
        end += (line.end - line.start).normalized() * 1000.0;
    }
    if !line.bounds.is_start_restricted() {
        start += (line.start - line.end).normalized() * 1000.0;
    }

    lines.push(GizmoLine { start, end, color });
}

fn draw_box(lines: &mut FfiVec<GizmoLine>, color: Vec4<f32>, corners: [Vec3<f32>; 8]) {
    // corner i is on the +extent side of x, y, z when bit 1, 2, 4 of i is set (`CollisionAabb::corners`),
    // so an edge joins two corners that differ in exactly one bit
    for i in 0..8 {
        for bit in [1, 2, 4] {
            if i & bit == 0 {
                lines.push(GizmoLine {
                    start: corners[i],
                    end: corners[i | bit],
                    color,
                });
            }
        }
    }
}

fn draw_sphere(lines: &mut FfiVec<GizmoLine>, color: Vec4<f32>, sphere: CollisionSphere) {
    let circle = |plane: fn(f32, f32) -> Vec3<f32>| -> Vec<Vec3<f32>> {
        (0..20)
            .map(|i| {
                let (a, b) = (i as f32 / 20.0 * TAU).sin_cos();
                sphere.center + plane(a, b) * sphere.radius
            })
            .collect()
    };

    draw_loop(lines, color, &circle(|a, b| Vec3::new(a, b, 0.0)));
    draw_loop(lines, color, &circle(|a, b| Vec3::new(a, 0.0, b)));
    draw_loop(lines, color, &circle(|a, b| Vec3::new(0.0, a, b)));
}

// Closed polyline: each point is joined to the next and the last to the first.
fn draw_loop(lines: &mut FfiVec<GizmoLine>, color: Vec4<f32>, points: &[Vec3<f32>]) {
    for (i, &start) in points.iter().enumerate() {
        let end = points[(i + 1) % points.len()];
        lines.push(GizmoLine { start, end, color });
    }
}
