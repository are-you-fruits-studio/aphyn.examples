//! The engine's showcase scene, a seamless 7 second loop: three boid flocks (450 entities) stream
//! around a lit berry under an orbiting camera, with emissive accents, bloom, exposure and ACES
//! grading, a normal-mapped floor, debug gizmo lines (axes and a bounding box) and a bitmap-font
//! overlay with the live entity count. Every mesh and texture is built in code, nothing is loaded
//! from disk. There are no controls.
//!
//! The simulation advances one fixed 1/60 s step per frame (the loop phase comes from
//! `TimeResource::frame_number`, `delta_time` is not used) from a fixed seed, so every run is the
//! same, and after a few loops each loop repeats the previous one. The boids are steered by ordered
//! ECS systems: a damped spring towards a target on a closed path, plus separation that fades out
//! before the loop seam so the last frame flows into the first.
//!
//! `--capture <dir>` plays 4 warm-up loops, then writes the 210 frames of one loop (30 fps) as PNG
//! files read back from the render target by a custom render feature, and exits. See the README.
//!
//! Modules: `scene` (materials, meshes, lights, camera, text, gizmos), `flock` (boid systems),
//! `mesh` (procedural meshes and the normal map), `capture` (the capture mode).
//!
//! Engine modules: `aphyn_ecs` (entities, components, resources, ordered systems), `aphyn_app`
//! (`TimeResource`), `aphyn_math` (vectors, quaternions, array helpers), `aphyn_transform`
//! (`LocalTransform`, UI rects), `aphyn_render` (mesh, texture and material assets, WGSL composer),
//! `aphyn_render_features` (materials from templates, lights, camera, bloom, exposure, color
//! grading, gizmos, custom render features), `aphyn_render_core` (GPU shader, texture readback),
//! `aphyn_ui` (bitmap-font text), `aphyn_asset_storage` (`AssetStorageResource`), `aphyn_window`
//! (`WindowState`: size, title, resizable).

mod capture;
mod flock;
mod mesh;
mod scene;

use std::path::PathBuf;

use aphyn::*;

pub const WINDOW_SIZE: [u32; 2] = [1280, 720];
pub const SIM_HZ: u32 = 60;
pub const SIM_DT: f32 = 1.0 / SIM_HZ as f32;
pub const LOOP_SECONDS: u32 = 7;
pub const LOOP_STEPS: u32 = SIM_HZ * LOOP_SECONDS;
pub const CAPTURE_FPS: u32 = 30;
pub const WARMUP_LOOPS: u32 = 4;

pub fn loop_phase(time: &TimeResource) -> f32 {
    (time.frame_number % LOOP_STEPS as u64) as f32 / LOOP_STEPS as f32
}

aphyn_entry_point!(init_app);

pub fn init_app(ctx: AppInitCtx) {
    let capture_dir = parse_capture_dir();

    let AppInitCtx {
        mut data,
        mut behavior,
        render,
        window,
    } = ctx;
    window.size = WINDOW_SIZE;
    window.is_resizable = false;
    window.title = "aphyn showcase".into();

    if let Some(dir) = &capture_dir {
        capture::register(data.as_mut(), behavior.as_mut(), render, dir.clone());
    }

    let mut behavior = behavior.as_mut();

    let mut start = behavior.get_mut(Schedule::Start);
    start.insert_system(scene::setup_post_processing);
    start.insert_system(scene::setup_scene);
    start.order_group(SYSTEM_GROUP_RENDER).before_system(scene::setup_scene);
    start.order_group(SYSTEM_GROUP_UI).before_system(scene::setup_scene);

    let mut update = behavior.get_mut(Schedule::Update);
    update.insert_system(flock::seek_targets);
    update.insert_system(flock::accumulate_separation);
    update.insert_system(flock::apply_steering);
    update.insert_system(flock::integrate_boids);
    update.insert_system(flock::move_flock_lights);
    update.insert_system(scene::orbit_camera);
    update.insert_system(scene::update_entity_count);
    update.insert_system(scene::draw_gizmos);

    update.order_system(flock::seek_targets).before_system(flock::apply_steering);
    update
        .order_system(flock::accumulate_separation)
        .before_system(flock::apply_steering);
    update
        .order_system(flock::apply_steering)
        .before_system(flock::integrate_boids);
    // systems that move entities run before the transform pass, so the frame renders their new pose
    update
        .order_system(flock::integrate_boids)
        .before_group(SYSTEM_GROUP_TRANSFORM);
    update
        .order_system(flock::move_flock_lights)
        .before_group(SYSTEM_GROUP_TRANSFORM);
    update.order_system(scene::orbit_camera).before_group(SYSTEM_GROUP_TRANSFORM);
    update.order_system(scene::update_entity_count).before_group(SYSTEM_GROUP_UI);
    update.order_system(scene::draw_gizmos).before_group(SYSTEM_GROUP_RENDER);

    if capture_dir.is_some() {
        update.insert_system(capture::measure_loop_seam);
        update
            .order_system(flock::integrate_boids)
            .before_system(capture::measure_loop_seam);
    }
}

fn parse_capture_dir() -> Option<PathBuf> {
    let mut args = std::env::args().skip(1);

    while let Some(arg) = args.next() {
        if arg == "--capture" {
            return Some(PathBuf::from(args.next().expect("--capture needs a directory")));
        }
    }

    None
}
