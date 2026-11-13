//! A looping music clip and its live waveform. The clip plays from the start, and every frame the
//! samples the audio device played last are drawn as gizmo lines across the window: dark purple
//! for the raw interleaved left/right samples, pink for the left/right average.
//!
//! The clip is `assets/ambient_loop.wav`, made by `tools/generate_clip.py`. The asset module loads
//! every `.asset` file in `Start`, so the clip is looked up by its asset key after
//! `SYSTEM_GROUP_ASSETS`. The waveform is the output mix the audio thread handed back through
//! `AudioStateResource::last_samples` (interleaved stereo at 48 kHz), so it shows what is actually
//! being played.
//!
//! Engine modules: `aphyn_audio` (`AudioSource`, `AudioClip`, `AudioStateResource`),
//! `aphyn_asset_loading` (`.asset` files), `aphyn_asset_storage` (`AssetStorageResource`),
//! `aphyn_render_features` (gizmos, camera), `aphyn_render` (`RenderSpace`), `aphyn_transform`,
//! `aphyn_ecs`.

use aphyn::*;

const CLIP_KEY: &str = "ambient_loop.asset";

const INTERLEAVED_COLOR: Vec4<f32> = Vec4::new(0.5, 0.0, 0.5, 1.0);
const AVERAGE_COLOR: Vec4<f32> = Vec4::new(1.0, 0.5, 1.0, 1.0);

aphyn_entry_point!(init_app);

pub fn init_app(ctx: AppInitCtx) {
    let AppInitCtx {
        mut data, mut behavior, ..
    } = ctx;

    let mut start = behavior.as_mut();
    let mut start = start.get_mut(Schedule::Start);
    start.insert_system(spawn_audio_source);
    start.order_group(SYSTEM_GROUP_ASSETS).before_system(spawn_audio_source);

    let mut update = behavior.as_mut();
    let mut update = update.get_mut(Schedule::Update);
    update.insert_system(draw_waveform);
    update.order_system(draw_waveform).before_group(SYSTEM_GROUP_RENDER);

    spawn_camera(data.as_mut());
}

fn spawn_camera(data: WorldDataMut) {
    let mut ec = data.into_entities_mut();

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

fn spawn_audio_source(mut world: WorldDataMut) {
    let (res, mut ec, _) = world.as_tuple_mut();

    let clip = res
        .get::<AssetStorageResource<AudioClip>>()
        .unwrap()
        .get_registered(CLIP_KEY)
        .unwrap()
        .clone();

    let source = ec.create_entity();
    ec.set_components(
        source,
        (AudioSource {
            clip,
            is_looped: true,
            ..Default::default()
        },),
    );
}

fn draw_waveform(audio: Res<AudioStateResource>, mut gizmos: ResMut<GizmosResource>) {
    let samples = audio.last_samples();

    if samples.len() <= 1 {
        return;
    }

    let lines = gizmos.space(RenderSpace::Clip);

    draw_polyline(lines, INTERLEAVED_COLOR, 0.5, samples.iter().copied());

    let averaged = samples
        .as_chunks::<AUDIO_CHANNELS_COUNT>()
        .0
        .iter()
        .map(|channels| channels.iter().sum::<f32>() / AUDIO_CHANNELS_COUNT as f32);
    draw_polyline(lines, AVERAGE_COLOR, 0.4, averaged);
}

// Spreads the values evenly from the left edge of the window (x = -1) to the right edge (x = 1),
// with each value as the height in clip space.
fn draw_polyline(lines: &mut FfiVec<GizmoLine>, color: Vec4<f32>, depth: f32, values: impl ExactSizeIterator<Item = f32>) {
    let step = 2.0 / (values.len().max(2) - 1) as f32;
    let points = values
        .enumerate()
        .map(|(i, y)| Vec3::new(-1.0 + i as f32 * step, y, depth))
        .collect::<Vec<_>>();

    for pair in points.windows(2) {
        lines.push(GizmoLine {
            start: pair[0],
            end: pair[1],
            color,
        });
    }
}
