//! An app whose only input is a local HTTP port. The window is filled with one flat color, and
//! another process on the same machine changes it with a request such as
//! `curl http://127.0.0.1:8787/color/ff0000`. There is no keyboard or mouse handling.
//!
//! A `tiny_http` server runs on its own thread, bound to 127.0.0.1 only (no authentication; it is
//! meant for local tools, test harnesses and scripts). It drops each requested color into a
//! one-slot mailbox, and an `Update` system paints the latest one on the next frame. The port is
//! 8787, or the value of the `HTTP_CONTROL_PORT` environment variable (`0` picks a free port, which
//! is printed at startup). The endpoints are listed in the example's README and at `GET /`.
//!
//! The color is the window's clear color: a small render feature clears the color target with it
//! at `RENDER_ORDER_PREPARE`, before anything else is drawn.
//!
//! Engine modules: `aphyn_render_features` (`RenderFeature`, `RENDER_ORDER_PREPARE`,
//! `CameraComponent`), `aphyn_render` (`CurrentRenderingResource`, `clear_color_target`, `Camera`),
//! `aphyn_transform` (camera transform), `aphyn_math` (color parsing), `aphyn_ecs`.

use aphyn::*;

mod color;
mod server;

use server::HttpControlResource;

// a dark grey, so a running app that has not been driven yet differs from a black window
const INITIAL_COLOR: [f32; 4] = [0.1, 0.1, 0.12, 1.0];

aphyn_entry_point!(init_app);

pub fn init_app(ctx: AppInitCtx) {
    let AppInitCtx {
        mut data,
        mut behavior,
        render,
        ..
    } = ctx;

    assert!(render.add_feature("http_control_clear", ClearFeature));
    data.as_mut()
        .resources_mut()
        .insert(ClearColorResource(color::srgb_to_linear(INITIAL_COLOR)));

    // bound here, before the window opens, so a busy port is reported up front; the app still runs
    match HttpControlResource::start(INITIAL_COLOR) {
        Ok(server) => {
            println!("http_control: listening on http://{}", server.address);
            println!("  curl http://{}/color/ff0000", server.address);
            data.as_mut().resources_mut().insert(server);
        }
        Err(e) => {
            println!("http_control: NOT LISTENING: {e}");
            println!(
                "  free the port or set {} to another one, then restart",
                server::PORT_VARIABLE
            );
        }
    }

    let mut update = behavior.as_mut();
    let mut update = update.get_mut(Schedule::Update);
    update.insert_system(apply_pending_color);
    update.order_system(apply_pending_color).before_group(SYSTEM_GROUP_RENDER);

    // render features run per camera view, so the clear needs a camera even with nothing to draw
    let mut ec = data.as_mut().into_entities_mut();
    let camera = ec.create_entity();
    ec.set_components(
        camera,
        (
            LocalTransform {
                position: Vec3::new(0.0, 0.0, -1.0),
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

// linear, as the HDR color target stores it
#[derive(Resource)]
struct ClearColorResource([f32; 4]);

struct ClearFeature;

impl RenderFeature for ClearFeature {
    fn add_passes(&mut self, _world: WorldDataMut, _thread_pool: ThreadPoolAccess, passes: &mut RenderPassWriter<Self>) {
        passes.add(RENDER_ORDER_PREPARE, |_, ctx| clear_window(ctx));
    }
}

fn clear_window(ctx: &mut RenderPassCtx) {
    let resources = ctx.world.resources();
    if let (Some(current), Some(clear)) = (
        resources.get::<CurrentRenderingResource>(),
        resources.get::<ClearColorResource>(),
    ) {
        clear_color_target(ctx.render_api, &current.target, clear.0);
    }
}

fn apply_pending_color(server: Option<Res<HttpControlResource>>, mut clear: ResMut<ClearColorResource>) {
    let Some(server) = server else {
        return;
    };
    let Some(rgba) = server.take_pending() else {
        return;
    };

    clear.0 = color::srgb_to_linear(rgba);
    server.set_applied(rgba);
    println!("[http_control] #{} applied {}", server.accepted_count(), color::to_hex(rgba));
}
