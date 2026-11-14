//! Click and drag selection of a few squares, and ping rings, built on the raw button state.
//!
//! - A left click hit-tests the squares: a hit selects only that square, a miss clears the selection.
//! - Once the cursor moves more than `DRAG_THRESHOLD` pixels with the left button held, the press
//!   becomes a drag: a marquee is drawn, and on release every square whose center is inside it
//!   becomes selected.
//! - A right click spawns a ping that grows and fades out over `PING_LIFETIME` seconds of
//!   `TimeResource::delta_time`, so its speed does not depend on the frame rate.

use aphyn::*;

use crate::{cursor_position, draw_loop, draw_ring};

const SQUARE_HALF_SIZE: f32 = 24.0;
const SQUARE_CENTERS: [Vec2<f32>; 5] = [
    Vec2::new(90.0, 90.0),
    Vec2::new(180.0, 90.0),
    Vec2::new(270.0, 90.0),
    Vec2::new(360.0, 90.0),
    Vec2::new(450.0, 90.0),
];
const SELECTION_OUTLINE_GAP: f32 = 5.0;

const DRAG_THRESHOLD: f32 = 6.0;

const PING_LIFETIME: f32 = 0.6;
const PING_MIN_RADIUS: f32 = 6.0;
const PING_MAX_RADIUS: f32 = 34.0;
const PING_SEGMENTS: usize = 32;

const SQUARE_IDLE: Vec4<f32> = Vec4::new(0.7, 0.75, 0.85, 1.0);
const SQUARE_SELECTED: Vec4<f32> = Vec4::new(1.0, 0.8, 0.15, 1.0);
const MARQUEE: Vec4<f32> = Vec4::new(0.3, 0.9, 1.0, 1.0);
const PING: Vec4<f32> = Vec4::new(1.0, 0.85, 0.2, 1.0);

struct Square {
    center: Vec2<f32>,
    selected: bool,
}

#[derive(Resource)]
pub(crate) struct Squares {
    items: Vec<Square>,
}

struct Ping {
    position: Vec2<f32>,
    age: f32,
}

#[derive(Resource, Default)]
pub(crate) struct Pings {
    items: Vec<Ping>,
}

#[derive(Resource, Default)]
pub(crate) struct DragState {
    is_pressed: bool,
    start: Vec2<f32>,
    current: Vec2<f32>,
    dragging: bool,
}

pub fn add_to(mut data: WorldDataMut, mut behavior: WorldBehaviorBuilderMut) {
    let mut resources = data.resources_mut();
    resources.insert(Squares {
        items: SQUARE_CENTERS
            .iter()
            .map(|&center| Square { center, selected: false })
            .collect(),
    });
    resources.insert(Pings::default());
    resources.insert(DragState::default());

    let mut update = behavior.get_mut(Schedule::Update);
    update.insert_system(update_selection);
    update.insert_system(spawn_pings);
    update.insert_system(update_pings);
    update.insert_system(draw_squares);
    update.insert_system(draw_marquee);
    update.insert_system(draw_pings);
    update.order_system(update_selection).before_system(draw_squares);
    update.order_system(update_selection).before_system(draw_marquee);
    update.order_system(spawn_pings).before_system(update_pings);
    update.order_system(update_pings).before_system(draw_pings);
    update.order_system(draw_squares).before_group(SYSTEM_GROUP_RENDER);
    update.order_system(draw_marquee).before_group(SYSTEM_GROUP_RENDER);
    update.order_system(draw_pings).before_group(SYSTEM_GROUP_RENDER);
}

fn point_in_square(point: Vec2<f32>, center: Vec2<f32>, half_size: f32) -> bool {
    (point.x - center.x).abs() <= half_size && (point.y - center.y).abs() <= half_size
}

fn point_in_rect(point: Vec2<f32>, a: Vec2<f32>, b: Vec2<f32>) -> bool {
    let (min_x, max_x) = (a.x.min(b.x), a.x.max(b.x));
    let (min_y, max_y) = (a.y.min(b.y), a.y.max(b.y));
    point.x >= min_x && point.x <= max_x && point.y >= min_y && point.y <= max_y
}

// Within one frame the button can be released and pressed again (or pressed and released), so the
// gesture in progress ends before a new one starts, and a gesture whose button is already up ends
// at once.
pub(crate) fn update_selection(input: Res<InputResource>, mut drag: ResMut<DragState>, mut squares: ResMut<Squares>) {
    let mouse = &input.mouse;
    let cursor = cursor_position(&input);

    if drag.is_pressed {
        drag.current = cursor;
        if (drag.current - drag.start).length_sq() > DRAG_THRESHOLD * DRAG_THRESHOLD {
            drag.dragging = true;
        }
        if mouse.is_just_released(MouseButton::Left) {
            finish_gesture(&mut drag, &mut squares);
        }
    }

    if mouse.is_just_pressed(MouseButton::Left) {
        *drag = DragState {
            is_pressed: true,
            start: cursor,
            current: cursor,
            dragging: false,
        };
        if !mouse.is_pressed(MouseButton::Left) {
            finish_gesture(&mut drag, &mut squares);
        }
    }
}

fn finish_gesture(drag: &mut DragState, squares: &mut Squares) {
    if drag.dragging {
        let mut count = 0;
        for square in &mut squares.items {
            square.selected = point_in_rect(square.center, drag.start, drag.current);
            count += usize::from(square.selected);
        }
        println!("marquee selected {count} square(s)");
    } else {
        let hit = squares
            .items
            .iter()
            .position(|square| point_in_square(drag.current, square.center, SQUARE_HALF_SIZE));
        for (i, square) in squares.items.iter_mut().enumerate() {
            square.selected = Some(i) == hit;
        }
        match hit {
            Some(i) => println!("click selected square {i}"),
            None => println!("click on empty space cleared the selection"),
        }
    }
    drag.is_pressed = false;
    drag.dragging = false;
}

pub(crate) fn spawn_pings(input: Res<InputResource>, mut pings: ResMut<Pings>) {
    if input.mouse.is_just_pressed(MouseButton::Right) {
        let position = cursor_position(&input);
        println!("ping at ({:.0}, {:.0})", position.x, position.y);
        pings.items.push(Ping { position, age: 0.0 });
    }
}

fn update_pings(time: Res<TimeResource>, mut pings: ResMut<Pings>) {
    for ping in &mut pings.items {
        ping.age += time.delta_time;
    }
    pings.items.retain(|ping| ping.age < PING_LIFETIME);
}

fn draw_squares(squares: Res<Squares>, mut gizmos: ResMut<GizmosResource>) {
    let lines = gizmos.space(RenderSpace::Window);
    for square in &squares.items {
        if square.selected {
            draw_square(lines, SQUARE_SELECTED, square.center, SQUARE_HALF_SIZE);
            // gizmos have no fill, so a second, larger outline marks the selection
            draw_square(
                lines,
                SQUARE_SELECTED,
                square.center,
                SQUARE_HALF_SIZE + SELECTION_OUTLINE_GAP,
            );
        } else {
            draw_square(lines, SQUARE_IDLE, square.center, SQUARE_HALF_SIZE);
        }
    }
}

fn draw_marquee(drag: Res<DragState>, mut gizmos: ResMut<GizmosResource>) {
    if !drag.dragging {
        return;
    }
    let (start, current) = (drag.start, drag.current);
    draw_loop(
        gizmos.space(RenderSpace::Window),
        MARQUEE,
        &[
            start.xyn(0.0),
            Vec3::new(current.x, start.y, 0.0),
            current.xyn(0.0),
            Vec3::new(start.x, current.y, 0.0),
        ],
    );
}

fn draw_pings(pings: Res<Pings>, mut gizmos: ResMut<GizmosResource>) {
    let lines = gizmos.space(RenderSpace::Window);
    for ping in &pings.items {
        let t = (ping.age / PING_LIFETIME).clamp(0.0, 1.0);
        let radius = lerp(PING_MIN_RADIUS, PING_MAX_RADIUS, t);
        // gizmo lines are drawn without blending, so alpha does nothing: fade towards the black background
        let color = (PING.xyz() * (1.0 - t)).xyzn(PING.w);
        draw_ring(lines, color, ping.position, radius, PING_SEGMENTS);
    }
}

fn draw_square(lines: &mut FfiVec<GizmoLine>, color: Vec4<f32>, center: Vec2<f32>, half_size: f32) {
    draw_loop(
        lines,
        color,
        &[
            Vec3::new(center.x - half_size, center.y - half_size, 0.0),
            Vec3::new(center.x + half_size, center.y - half_size, 0.0),
            Vec3::new(center.x + half_size, center.y + half_size, 0.0),
            Vec3::new(center.x - half_size, center.y + half_size, 0.0),
        ],
    );
}
