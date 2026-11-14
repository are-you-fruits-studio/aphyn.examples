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
//! The engine always clears the window to black, so the color is a UI rect the size of the window,
//! recolored through its `ImageComponent`.
//!
//! Engine modules: `aphyn_ui` (`ImageComponent`), `aphyn_render_features` (UI material template,
//! `MaterialComponent`, `BatchedMeshComponent`, `CameraComponent`), `aphyn_render` (`Material`,
//! `Camera`), `aphyn_transform` (rect and camera transforms), `aphyn_asset_storage`, `aphyn_math`
//! (color parsing), `aphyn_ecs`.

use aphyn::*;

mod color;
mod server;

use server::HttpControlResource;

// a dark grey, so a running app that has not been driven yet differs from the black clear color
const INITIAL_COLOR: [f32; 4] = [0.1, 0.1, 0.12, 1.0];

aphyn_entry_point!(init_app);

pub fn init_app(ctx: AppInitCtx) {
    let AppInitCtx {
        mut data, mut behavior, ..
    } = ctx;

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

    let mut start = behavior.as_mut();
    let mut start = start.get_mut(Schedule::Start);
    start.insert_system(spawn_screen_fill);
    start.order_group(SYSTEM_GROUP_RENDER).before_system(spawn_screen_fill);

    let mut update = behavior.as_mut();
    let mut update = update.get_mut(Schedule::Update);
    update.insert_system(apply_pending_color);
    update.order_system(apply_pending_color).before_group(SYSTEM_GROUP_UI);

    // nothing renders without a camera, even when everything on screen is UI
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

#[derive(Component)]
struct ScreenFill;

fn spawn_screen_fill(mut world: WorldDataMut) {
    // one white material; the rect's color comes from ImageComponent.color
    let material = create_material_from_template_key(
        world.as_ref().resources(),
        BUILTIN_TEMPLATE_UI_OPAQUE_UNLIT,
        MaterialData::new(),
    )
    .unwrap();
    let material = world
        .as_mut()
        .resources_mut()
        .get_mut::<AssetStorageResource<Material>>()
        .unwrap()
        .insert(material);

    let mut ec = world.entities_mut();
    let fill = ec.create_entity();
    ec.set_components(
        fill,
        (
            ScreenFill,
            LocalRectComponent {
                anchor: Vec2::splat(0.5),
                pivot: Vec2::splat(0.5),
                // relative to the window, so it stays full size when the window is resized
                scale: Vec2::new(UiVal::vw(1.0).into(), UiVal::vh(1.0).into()),
                z: -10.0,
                ..Default::default()
            },
            BatchedMeshComponent::default(),
            MaterialComponent { material },
            ImageComponent {
                color: Vec4::from_array(INITIAL_COLOR),
                ..Default::default()
            },
        ),
    );
}

// The opaque UI template discards fragments with alpha below its `alpha_threshold` (0.5), so an
// alpha byte under 0x80 leaves the window black and anything from 0x80 up draws fully opaque.
fn apply_pending_color(
    server: Option<Res<HttpControlResource>>,
    mut fills: WorldQuery<&mut ImageComponent, WithFilter<ScreenFill>>,
) {
    let Some(server) = server else {
        return;
    };
    let Some(rgba) = server.take_pending() else {
        return;
    };

    for image in fills.iter_mut() {
        image.color = Vec4::from_array(rgba);
    }
    server.set_applied(rgba);
    println!("[http_control] #{} applied {}", server.accepted_count(), color::to_hex(rgba));
}
