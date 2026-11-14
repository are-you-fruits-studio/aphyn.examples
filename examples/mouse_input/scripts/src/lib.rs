//! Mouse input drawn with gizmo lines. A white circle chases the cursor and takes the color of the
//! held button, and a grey border around the window takes the color of the last pressed button.
//! A row of five squares near the top-left corner can be selected with clicks and a drag marquee,
//! and a right click leaves a ping ring that grows and fades out (see `selection.rs`).
//!
//! Controls:
//! - move the mouse: the circle follows the cursor with easing
//! - mouse wheel: grows or shrinks the circle, within 10 to 160 pixels of radius
//! - hold Left / Right / Middle: the circle turns green / red / blue; a press also recolors the border
//! - Left click on a square: selects it alone; a click on empty space clears the selection
//! - Left drag: draws a marquee; on release, selects every square whose center is inside it
//! - Right click: spawns a ping ring at the cursor
//!
//! Every press and release is printed to the console with the cursor position, and so are wheel
//! turns (in lines for a notched wheel, in pixels for a touchpad), pings and selection changes.
//!
//! The cursor position is in window pixels (origin top-left, y down), so the circle, the squares and
//! the rings are drawn in `RenderSpace::Window`, where gizmo coordinates are pixels too. The border
//! is drawn in `RenderSpace::Clip` so it follows the window size. There is no built-in 2D picking:
//! hit tests are plain rectangle checks against the cursor position. The circle eases towards the
//! cursor by the same amount per second at any frame rate, using `TimeResource::delta_time`.
//!
//! Engine modules: `aphyn_app` (`InputResource`, `MouseButton`, `TimeResource`),
//! `aphyn_render_features` (`GizmosResource`, `GizmoLine`, `CameraComponent`), `aphyn_render`
//! (`Camera`, `CameraProjection`, `RenderSpace`), `aphyn_transform` (`LocalTransform`), `aphyn_math`
//! (`Vec2`, `lerp`, swizzles), `aphyn_ecs`.

use std::f32::consts::TAU;

use aphyn::*;

mod selection;

const CIRCLE_RADIUS: f32 = 40.0;
const CIRCLE_MIN_RADIUS: f32 = 10.0;
const CIRCLE_MAX_RADIUS: f32 = 160.0;
const CIRCLE_RADIUS_PER_WHEEL_LINE: f32 = 5.0;
const CIRCLE_SEGMENTS: usize = 48;
// share of the remaining distance the circle covers in one frame at 60 fps
const CIRCLE_EASING: f32 = 0.15;
// border corners in clip space, just inside the window edges
const BORDER_INSET: f32 = 0.98;

const WHITE: Vec4<f32> = Vec4::new(1.0, 1.0, 1.0, 1.0);
const GREEN: Vec4<f32> = Vec4::new(0.2, 1.0, 0.35, 1.0);
const RED: Vec4<f32> = Vec4::new(1.0, 0.25, 0.2, 1.0);
const BLUE: Vec4<f32> = Vec4::new(0.3, 0.5, 1.0, 1.0);
const GREY: Vec4<f32> = Vec4::new(0.3, 0.3, 0.3, 1.0);

aphyn_entry_point!(init_app);

#[derive(Resource)]
struct Circle {
    position: Vec2<f32>,
    radius: f32,
    color: Vec4<f32>,
}

#[derive(Resource)]
struct Border {
    color: Vec4<f32>,
}

pub fn init_app(ctx: AppInitCtx) {
    let AppInitCtx {
        mut data, mut behavior, ..
    } = ctx;

    let mut resources = data.as_mut().into_resources_mut();
    resources.insert(Circle {
        position: Vec2::new(0.0, 0.0),
        radius: CIRCLE_RADIUS,
        color: WHITE,
    });
    resources.insert(Border { color: GREY });

    let mut update = behavior.as_mut();
    let mut update = update.get_mut(Schedule::Update);
    update.insert_system(update_circle);
    update.insert_system(draw_circle);
    update.insert_system(draw_border);
    update.insert_system(print_mouse_events);
    update.order_system(update_circle).before_system(draw_circle);
    update.order_system(update_circle).before_system(draw_border);
    update.order_system(draw_circle).before_group(SYSTEM_GROUP_RENDER);
    update.order_system(draw_border).before_group(SYSTEM_GROUP_RENDER);
    // print a button press before what it did
    update
        .order_system(print_mouse_events)
        .before_system(selection::update_selection);
    update.order_system(print_mouse_events).before_system(selection::spawn_pings);

    selection::add_to(data.as_mut(), behavior.as_mut());

    // nothing renders without a camera, even when everything on screen is window-space gizmos
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

fn cursor_position(input: &InputResource) -> Vec2<f32> {
    Vec2::new(input.mouse.position[0] as f32, input.mouse.position[1] as f32)
}

fn update_circle(input: Res<InputResource>, time: Res<TimeResource>, mut circle: ResMut<Circle>, mut border: ResMut<Border>) {
    let mouse = &input.mouse;

    let easing = 1.0 - (1.0 - CIRCLE_EASING).powf(time.delta_time * 60.0);
    circle.position = circle.position.lerp(cursor_position(&input), easing);

    // touchpads scroll in pixels: one pixel of scroll changes the radius by one pixel
    let wheel = mouse.scroll_delta_lines[1] * CIRCLE_RADIUS_PER_WHEEL_LINE + mouse.scroll_delta_pixels[1] as f32;
    circle.radius = (circle.radius + wheel).clamp(CIRCLE_MIN_RADIUS, CIRCLE_MAX_RADIUS);

    circle.color = if mouse.is_pressed(MouseButton::Left) {
        GREEN
    } else if mouse.is_pressed(MouseButton::Right) {
        RED
    } else if mouse.is_pressed(MouseButton::Middle) {
        BLUE
    } else {
        WHITE
    };

    for (button, color) in [
        (MouseButton::Left, GREEN),
        (MouseButton::Right, RED),
        (MouseButton::Middle, BLUE),
    ] {
        if mouse.is_just_pressed(button) {
            border.color = color;
        }
    }
}

fn draw_circle(circle: Res<Circle>, mut gizmos: ResMut<GizmosResource>) {
    draw_ring(
        gizmos.space(RenderSpace::Window),
        circle.color,
        circle.position,
        circle.radius,
        CIRCLE_SEGMENTS,
    );
}

fn draw_border(border: Res<Border>, mut gizmos: ResMut<GizmosResource>) {
    let k = BORDER_INSET;
    draw_loop(
        gizmos.space(RenderSpace::Clip),
        border.color,
        &[
            Vec3::new(-k, -k, 0.0),
            Vec3::new(k, -k, 0.0),
            Vec3::new(k, k, 0.0),
            Vec3::new(-k, k, 0.0),
        ],
    );
}

fn print_mouse_events(input: Res<InputResource>) {
    let mouse = &input.mouse;
    let [x, y] = mouse.position;

    for (button, name) in [
        (MouseButton::Left, "Left"),
        (MouseButton::Right, "Right"),
        (MouseButton::Middle, "Middle"),
    ] {
        let mut events = [
            (mouse.is_just_pressed(button), "pressed"),
            (mouse.is_just_released(button), "released"),
        ];
        // a release and a press in the same frame: the button is still held, so the release came first
        if mouse.is_pressed(button) {
            events.reverse();
        }
        for (_, event) in events.into_iter().filter(|&(happened, _)| happened) {
            println!("{name} {event} at ({x:.0}, {y:.0})");
        }
    }

    let [lines_x, lines_y] = mouse.scroll_delta_lines;
    let [pixels_x, pixels_y] = mouse.scroll_delta_pixels;
    if lines_x != 0.0 || lines_y != 0.0 || pixels_x != 0.0 || pixels_y != 0.0 {
        println!("wheel: ({lines_x}, {lines_y}) lines, ({pixels_x}, {pixels_y}) pixels");
    }
}

fn draw_ring(lines: &mut FfiVec<GizmoLine>, color: Vec4<f32>, center: Vec2<f32>, radius: f32, segments: usize) {
    let points: Vec<_> = (0..segments)
        .map(|i| {
            let (sin, cos) = (TAU * i as f32 / segments as f32).sin_cos();
            (center + Vec2::new(cos, sin) * radius).xyn(0.0)
        })
        .collect();
    draw_loop(lines, color, &points);
}

fn draw_loop(lines: &mut FfiVec<GizmoLine>, color: Vec4<f32>, points: &[Vec3<f32>]) {
    for (i, &start) in points.iter().enumerate() {
        let end = points[(i + 1) % points.len()];
        lines.push(GizmoLine { start, end, color });
    }
}
