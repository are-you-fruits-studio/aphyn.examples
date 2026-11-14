//! Sound effects played from the keyboard: one-shots, a loop, several clips mixed at once, a stereo
//! clip (left, right, both), and mono 44.1 kHz clips that the loader converts to the engine's 48 kHz
//! stereo. The window shows the output the audio device played last as two green traces, the left
//! channel above the right one, and every start and stop of a sound is printed to the console,
//! including a one-shot stopping by itself at its end.
//!
//! Controls (also printed on startup):
//! - `0` stops every sound.
//! - `1` plays a short blip, from the start again if it is still playing.
//! - `2` starts or stops a looping hum.
//! - `3` plays the blip on five sources at once. They are mixed by adding their samples, so each
//!   plays at volume 1/5 to keep the sum below full scale.
//! - `4` plays a 2 s mono clip recorded at 44.1 kHz.
//! - `5` plays a short melody.
//! - `6` plays a stereo clip: a tone on the left only, then another on the right only, then both.
//! - `7` plays a 0.5 s mono clip recorded at 44.1 kHz.
//!
//! Playback is driven by the `AudioSource` component alone: `is_playing` starts and stops a sound,
//! `is_looped` repeats it, and `playback_time` with `should_force_playback_time` seeks (here: back
//! to the start), and `volume` scales it. The sources are changed before `SYSTEM_GROUP_AUDIO`, so a
//! key press reaches the audio device in the same frame. The clips are `assets/*.wav`, made by
//! `tools/generate_assets.py`, and are looked up by their asset key after `SYSTEM_GROUP_ASSETS` has
//! loaded every `.asset` file.
//!
//! Engine modules: `aphyn_audio` (`AudioSource`, `AudioClip`, `AudioStateResource`),
//! `aphyn_asset_loading` (`.asset` files, WAV conversion, `AssetsDirResource` in `main.rs`),
//! `aphyn_asset_storage` (`AssetStorageResource`), `aphyn_app` (`InputResource`),
//! `aphyn_render_features` (`GizmosResource`, `CameraComponent`), `aphyn_render` (`Camera`,
//! `CameraProjection`, `RenderSpace`), `aphyn_transform` (`LocalTransform`), `aphyn_ecs`.

use std::collections::HashMap;

use aphyn::*;

const STRESS_SOURCES_COUNT: usize = 5;
const WAVEFORM_COLOR: Vec4<f32> = Vec4::new(0.3, 1.0, 0.4, 1.0);

aphyn_entry_point!(init_app);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Sound {
    OneShot,
    Loop,
    Stress,
    Mono,
    Melody,
    Stereo,
    MonoShort,
}

impl Sound {
    fn key(self) -> KeyCode {
        match self {
            Sound::OneShot => KeyCode::Digit1,
            Sound::Loop => KeyCode::Digit2,
            Sound::Stress => KeyCode::Digit3,
            Sound::Mono => KeyCode::Digit4,
            Sound::Melody => KeyCode::Digit5,
            Sound::Stereo => KeyCode::Digit6,
            Sound::MonoShort => KeyCode::Digit7,
        }
    }

    fn clip_key(self) -> &'static str {
        match self {
            Sound::OneShot | Sound::Stress => "blip.asset",
            Sound::Loop => "hum.asset",
            Sound::Mono => "mono.asset",
            Sound::Melody => "melody.asset",
            Sound::Stereo => "stereo.asset",
            Sound::MonoShort => "mono_short.asset",
        }
    }

    fn volume(self) -> f32 {
        match self {
            Sound::Stress => 1.0 / STRESS_SOURCES_COUNT as f32,
            _ => 1.0,
        }
    }
}

#[derive(Component)]
struct SoundComponent(Sound);

pub fn init_app(ctx: AppInitCtx) {
    let AppInitCtx {
        mut data, mut behavior, ..
    } = ctx;

    let mut start = behavior.as_mut();
    let mut start = start.get_mut(Schedule::Start);
    start.insert_system(spawn_sources);
    start.order_group(SYSTEM_GROUP_ASSETS).before_system(spawn_sources);

    let mut update = behavior.as_mut();
    let mut update = update.get_mut(Schedule::Update);
    update.insert_system(control_sources);
    update.order_system(control_sources).before_group(SYSTEM_GROUP_AUDIO);
    update.insert_system(log_transitions);
    update.insert_system(draw_waveform);
    update.order_system(draw_waveform).before_group(SYSTEM_GROUP_RENDER);

    spawn_camera(data.as_mut());
}

// Nothing is drawn without a camera, not even clip-space gizmos.
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

fn spawn_sources(mut world: WorldDataMut) {
    let (res, mut ec, _) = world.as_tuple_mut();
    let clips = res.get::<AssetStorageResource<AudioClip>>().unwrap();

    let sounds = [
        Sound::OneShot,
        Sound::Loop,
        Sound::Mono,
        Sound::Melody,
        Sound::Stereo,
        Sound::MonoShort,
    ]
    .into_iter()
    .chain([Sound::Stress; STRESS_SOURCES_COUNT]);

    for sound in sounds {
        let key = sound.clip_key();
        let clip = clips
            .get_registered(key)
            .unwrap_or_else(|| panic!("audio asset not loaded: assets/{key}"))
            .clone();

        let source = ec.create_entity();
        ec.set_components(
            source,
            (
                AudioSource {
                    clip,
                    volume: sound.volume(),
                    is_playing: false,
                    is_looped: sound == Sound::Loop,
                    ..Default::default()
                },
                SoundComponent(sound),
            ),
        );
    }

    println!("Sound effects, press a number key:");
    println!("  0  stop all");
    println!("  1  blip (one-shot)");
    println!("  2  start / stop the looping hum");
    println!("  3  {STRESS_SOURCES_COUNT} blips at once, each at volume 1/{STRESS_SOURCES_COUNT}");
    println!("  4  mono 44.1 kHz clip, 2 s");
    println!("  5  melody");
    println!("  6  stereo: left, then right, then both");
    println!("  7  mono 44.1 kHz clip, 0.5 s");
}

fn control_sources(input: Res<InputResource>, mut sources: WorldQuery<(&SoundComponent, &mut AudioSource)>) {
    let stop_all = input.keyboard.is_just_pressed(KeyCode::Digit0);

    for (sound, source) in sources.iter_mut() {
        if input.keyboard.is_just_pressed(sound.0.key()) {
            if sound.0 == Sound::Loop && source.is_playing {
                source.is_playing = false;
            } else {
                source.is_playing = true;
                source.playback_time = 0.0;
                source.should_force_playback_time = true;
            }
        }

        if stop_all {
            source.is_playing = false;
        }
    }
}

// Also catches the audio module stopping a one-shot that reached its end.
fn log_transitions(mut was_playing: Local<HashMap<EntityId, bool>>, sources: WorldQuery<(EntityId, &SoundComponent, &AudioSource)>) {
    for (entity, sound, source) in sources.iter() {
        let was = was_playing.insert(entity, source.is_playing).unwrap_or(false);
        if was != source.is_playing {
            println!("{:?} {}", sound.0, if source.is_playing { "started" } else { "stopped" });
        }
    }
}

// The samples the device played last are interleaved left/right: the left channel is drawn in the
// upper half of the window, the right one in the lower half.
fn draw_waveform(audio: Res<AudioStateResource>, mut gizmos: ResMut<GizmosResource>) {
    let samples = audio.last_samples();
    let frames = samples.len() / AUDIO_CHANNELS_COUNT;
    if frames <= 1 {
        return;
    }

    let step = 2.0 / (frames - 1) as f32;
    let lines = gizmos.space(RenderSpace::Clip);

    for (channel, center) in [(0, 0.5), (1, -0.5)] {
        let point = |frame: usize| {
            let sample = samples[frame * AUDIO_CHANNELS_COUNT + channel];
            Vec3::new(-1.0 + frame as f32 * step, center + sample * 0.5, 0.5)
        };
        for frame in 1..frames {
            lines.push(GizmoLine {
                start: point(frame - 1),
                end: point(frame),
                color: WAVEFORM_COLOR,
            });
        }
    }
}
