//! Text in, babble out. Type into the single-line field at the bottom of the window and press Enter:
//! the typed letters go through a small synthesizer (`speech.rs`) that turns them into a short burst
//! of gibberish "speech", the clip plays, and the green circle pulses with its loudness.
//!
//! Controls: printable ASCII keys type, Backspace deletes the last character, Enter (or keypad
//! Enter) plays the current text. The field keeps its text, so you can edit a word and play it
//! again. Other characters are ignored, because the built-in bitmap fonts only have ASCII glyphs.
//! Drag the four bars in the top-left corner to change the next clip: SPEED, PITCH, VIBRA (pitch
//! wobble) and BRIGHT (overtones).
//!
//! Shows reading typed text through `TextInputEvent`, building an `AudioClip` from samples at
//! runtime and playing it with an `AudioSource`, measuring the played output through
//! `AudioStateResource::last_samples`, a texture made from bytes (the circle is a square rect with a
//! round alpha mask: the default `alpha_threshold` of 0.5 cuts the corners away), and
//! `TextComponent` rects used as mouse-draggable sliders.
//!
//! Engine modules: `aphyn_app` (`TextInputEvent`, `InputResource`, `TimeResource`), `aphyn_audio`
//! (`AudioClip`, `AudioSource`, `AudioStateResource`), `aphyn_ui` (`TextComponent`, `ImageComponent`,
//! built-in fonts), `aphyn_transform` (`LocalRectComponent`, `GlobalRectComponent`, `UiVal`,
//! `LocalTransform`), `aphyn_render` (`RenderApiResource::create_texture`, materials, `Camera`),
//! `aphyn_render_features` (UI material template, `MaterialComponent`, `BatchedMeshComponent`,
//! `CameraComponent`), `aphyn_asset_storage`, `aphyn_ecs`.

use std::collections::HashSet;

use aphyn::*;

mod speech;

aphyn_entry_point!(init_app);

// The circle's resting diameter in window heights; `vh` on both axes keeps it round at any aspect ratio.
const CIRCLE_SIZE: f32 = 0.28;
const CIRCLE_ANCHOR_X: f32 = 0.68;
const CIRCLE_TEXTURE_SIZE: u32 = 128;
const CIRCLE_COLOR: Vec4<f32> = Vec4::new(0.2, 0.9, 0.3, 1.0);
const CIRCLE_PULSE_GAIN: f32 = 2.0;
const CIRCLE_PULSE_MAX: f32 = 1.8;
// share of the previous loudness kept per 1/60 s, so the pulse does not jitter
const PULSE_SMOOTHING: f32 = 0.7;

// The SPEED slider reads 1.00x at half the synthesizer's own pace.
const BASE_SPEED_SCALE: f32 = 0.5;

// The 8x8 font has square glyphs and the text size is the rect height, so one character is
// `ROW_HEIGHT` window heights wide.
const ROW_HEIGHT: f32 = 0.032;
const ROW_STEP: f32 = 0.06;
const FIRST_ROW_Y: f32 = 0.06;
const LEFT_MARGIN: f32 = 0.03;
const LABEL_CHARS: usize = 6;
const BAR_CHARS: usize = 14;
const READOUT_CHARS: usize = 5;

const FIELD_COLOR: Vec4<f32> = Vec4::new(0.95, 0.95, 0.95, 1.0);
const DIM_COLOR: Vec4<f32> = Vec4::new(0.6, 0.6, 0.65, 1.0);

#[derive(Clone, Copy, PartialEq, Eq)]
enum Param {
    Speed,
    Pitch,
    Vibrato,
    Brightness,
}

impl Param {
    const ALL: [Param; 4] = [Param::Speed, Param::Pitch, Param::Vibrato, Param::Brightness];

    fn label(self) -> &'static str {
        match self {
            Param::Speed => "SPEED",
            Param::Pitch => "PITCH",
            Param::Vibrato => "VIBRA",
            Param::Brightness => "BRIGHT",
        }
    }

    fn range(self) -> (f32, f32) {
        match self {
            Param::Speed => (0.4, 2.5),
            Param::Pitch => (0.5, 2.0),
            Param::Vibrato => (0.0, 3.0),
            Param::Brightness => (0.0, 2.5),
        }
    }

    fn bar_text(self, value: f32) -> String {
        let (min, max) = self.range();
        let progress = ((value - min) / (max - min)).clamp(0.0, 1.0);
        let filled = ((progress * BAR_CHARS as f32).round() as usize).min(BAR_CHARS);
        format!("[{}{}]", "#".repeat(filled), "-".repeat(BAR_CHARS - filled))
    }
}

// Slider values in `Param::ALL` order; 1.0 is the plain voice for each of them.
#[derive(Resource)]
struct SynthSettings([f32; 4]);

impl SynthSettings {
    fn get(&self, param: Param) -> f32 {
        self.0[param as usize]
    }
}

#[derive(Component)]
struct GreenCircle;

#[derive(Component)]
struct InputField;

#[derive(Component)]
struct Voice;

// The draggable `[####------]` part of a slider row; only this rect reacts to the mouse.
#[derive(Component)]
struct SliderBar(Param);

#[derive(Component)]
struct SliderReadout(Param);

pub fn init_app(ctx: AppInitCtx) {
    let AppInitCtx {
        mut data, mut behavior, ..
    } = ctx;

    data.as_mut().resources_mut().insert(SynthSettings([1.0; 4]));

    let mut start = behavior.as_mut();
    let mut start = start.get_mut(Schedule::Start);
    start.insert_system(spawn_scene);
    // the materials need the render module, the font comes from the UI module
    start.order_group(SYSTEM_GROUP_RENDER).before_system(spawn_scene);
    start.order_group(SYSTEM_GROUP_UI).before_system(spawn_scene);

    let mut update = behavior.as_mut();
    let mut update = update.get_mut(Schedule::Update);
    update.insert_system(type_text);
    update.insert_system(play_babble);
    update.insert_system(pulse_circle);
    update.insert_system(drag_sliders);
    update.insert_system(show_slider_values);
    update.order_system(play_babble).before_group(SYSTEM_GROUP_AUDIO);
    update.order_system(type_text).before_group(SYSTEM_GROUP_UI);
    update.order_system(drag_sliders).before_group(SYSTEM_GROUP_UI);
    update.order_system(show_slider_values).before_group(SYSTEM_GROUP_UI);

    // nothing renders without a camera, even when everything on screen is UI
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

fn spawn_scene(mut world: WorldDataMut) {
    let ui_assets = world.as_ref().resources().get::<StandardUiAssetsResource>().unwrap();
    let font = ui_assets.font_px_8_8.clone();
    let atlas = ui_assets.texture_text_px_8_8.clone();

    let circle_texture = world.as_ref().resources().get::<RenderApiResource>().unwrap().create_texture(
        FilterMode::Linear,
        [CIRCLE_TEXTURE_SIZE; 2],
        &circle_mask(),
        Default::default(),
    );
    let circle_texture = world
        .resources_mut()
        .get_mut::<AssetStorageResource<StandardTexture>>()
        .unwrap()
        .insert(circle_texture);

    let circle_material = ui_material(&mut world, circle_texture);
    let text_material = ui_material(&mut world, atlas);

    let mut ec = world.entities_mut();

    // The circle texture is white; ImageComponent.color makes it green.
    let (rect, mesh, material) = ui_rect(
        &circle_material,
        LocalRectComponent {
            anchor: Vec2::new(CIRCLE_ANCHOR_X, 0.5),
            scale: Vec2::splat(UiVal::vh(CIRCLE_SIZE).into()),
            z: -10.0,
            ..Default::default()
        },
    );
    let circle = ec.create_entity();
    ec.set_components(
        circle,
        (
            GreenCircle,
            rect,
            mesh,
            material,
            ImageComponent {
                color: CIRCLE_COLOR,
                ..Default::default()
            },
        ),
    );

    let (rect, mesh, material) = ui_rect(
        &text_material,
        LocalRectComponent {
            anchor: Vec2::new(0.5, 0.93),
            scale: Vec2::new(UiVal::vw(0.9).into(), UiVal::vh(0.08).into()),
            z: -11.0,
            ..Default::default()
        },
    );
    let field = ec.create_entity();
    ec.set_components(field, (InputField, rect, mesh, material, text(&font, "", FIELD_COLOR)));

    // Each slider row is a label, the bar and a readout, laid out in character widths from the left
    // margin. The offsets are in `vh` like the glyphs, so the row stays together when the window is resized.
    for (row, param) in Param::ALL.into_iter().enumerate() {
        let y = FIRST_ROW_Y + row as f32 * ROW_STEP;
        // one blank character between the parts; the bar is `[`, BAR_CHARS marks and `]`
        let bar_column = LABEL_CHARS + 1;
        let readout_column = bar_column + BAR_CHARS + 2 + 1;

        let slider_rect = |column: usize, chars: usize| {
            ui_rect(
                &text_material,
                LocalRectComponent {
                    anchor: Vec2::new(LEFT_MARGIN, y),
                    offset: Vec2::new(UiVal::vh(column as f32 * ROW_HEIGHT), UiVal::px(0.0)),
                    pivot: Vec2::splat(0.0),
                    scale: Vec2::new(UiVal::vh(chars as f32 * ROW_HEIGHT).into(), UiVal::vh(ROW_HEIGHT).into()),
                    z: -11.0,
                    ..Default::default()
                },
            )
        };

        let (rect, mesh, material) = slider_rect(0, LABEL_CHARS);
        let label = ec.create_entity();
        ec.set_components(label, (rect, mesh, material, text(&font, param.label(), DIM_COLOR)));

        let (rect, mesh, material) = slider_rect(bar_column, BAR_CHARS + 2);
        let bar = ec.create_entity();
        let bar_text = text(&font, &param.bar_text(1.0), FIELD_COLOR);
        ec.set_components(bar, (SliderBar(param), rect, mesh, material, bar_text));

        let (rect, mesh, material) = slider_rect(readout_column, READOUT_CHARS);
        let readout = ec.create_entity();
        let readout_text = text(&font, "", DIM_COLOR);
        ec.set_components(readout, (SliderReadout(param), rect, mesh, material, readout_text));
    }

    let voice = ec.create_entity();
    ec.set_components(
        voice,
        (
            Voice,
            AudioSource {
                is_playing: false,
                ..Default::default()
            },
        ),
    );
}

// White RGBA pixels, opaque inside the circle and transparent outside it.
fn circle_mask() -> Vec<u8> {
    let size = CIRCLE_TEXTURE_SIZE;
    let center = (size as f32 - 1.0) / 2.0;
    let radius = size as f32 / 2.0;

    (0..size * size)
        .flat_map(|i| {
            let dx = (i % size) as f32 - center;
            let dy = (i / size) as f32 - center;
            let alpha = if (dx * dx + dy * dy).sqrt() <= radius { 255 } else { 0 };
            [255, 255, 255, alpha]
        })
        .collect()
}

fn ui_material(world: &mut WorldDataMut, texture: AssetHandle<StandardTexture>) -> AssetHandle<Material> {
    let mut data = MaterialData::new();
    data.insert("color_tex", MaterialDataValue::Texture(texture));
    let material = create_material_from_template_key(world.as_ref().resources(), BUILTIN_TEMPLATE_UI_OPAQUE_UNLIT, data).unwrap();

    world
        .resources_mut()
        .get_mut::<AssetStorageResource<Material>>()
        .unwrap()
        .insert(material)
}

fn ui_rect(
    material: &AssetHandle<Material>,
    rect: LocalRectComponent,
) -> (LocalRectComponent, BatchedMeshComponent, MaterialComponent) {
    (
        rect,
        BatchedMeshComponent::default(),
        MaterialComponent {
            material: material.clone(),
        },
    )
}

fn text(font: &AssetHandle<Font>, text: &str, color: Vec4<f32>) -> TextComponent {
    TextComponent {
        font: font.clone(),
        text: text.into(),
        font_size: UiVal::ph(1.0),
        horizontal_align: HorizontalAlign::Left,
        vertical_align: VerticalAlign::Middle,
        // UI space is y-down
        is_y_inverted: true,
        horizontal_spacing: UiVal::px(0.0),
        color,
    }
}

// Aphyn v0.21.1 sends a `TextInputEvent` for every key event that carries text, without checking
// that the key was pressed. On Linux (X11 and Wayland) releases carry text too; on Windows and macOS
// only presses do. A held key does not repeat, so on Linux each release is paired with its press by
// lowercase text and skipped ("H" pressed, Shift released, "h" released). A shifted symbol whose
// Shift is released first (`!` then `1`) is still typed twice, and the next press of either
// symbol can then be dropped. Remove this workaround once the engine sends text for presses only.
const RELEASES_SEND_TEXT: bool = cfg!(all(unix, not(target_vendor = "apple"), not(target_os = "android")));

#[derive(Default)]
struct PendingReleases(HashSet<String>);

fn type_text(
    text_input: Evt<TextInputEvent>,
    mut pending_releases: Local<PendingReleases>,
    mut fields: WorldQuery<&mut TextComponent, WithFilter<InputField>>,
) {
    for event in text_input.iter() {
        let typed = event.0.as_str();

        if RELEASES_SEND_TEXT {
            let key = typed.to_lowercase();
            if pending_releases.0.remove(&key) {
                continue;
            }
            pending_releases.0.insert(key);
        }

        for field in fields.iter_mut() {
            if typed == "\u{8}" {
                let mut text = field.text.to_string();
                text.pop();
                field.text = text.into();
            } else if typed.chars().all(|c| c.is_ascii() && !c.is_control()) {
                field.text.push_str(typed);
            }
        }
    }
}

fn play_babble(
    input: Res<InputResource>,
    settings: Res<SynthSettings>,
    fields: WorldQuery<&TextComponent, WithFilter<InputField>>,
    mut voices: WorldQuery<&mut AudioSource, WithFilter<Voice>>,
    mut clips: ResMut<AssetStorageResource<AudioClip>>,
    mut audio_state: ResMut<AudioStateResource>,
) {
    let keyboard = &input.keyboard;
    if !keyboard.is_just_pressed(KeyCode::Enter) && !keyboard.is_just_pressed(KeyCode::NumpadEnter) {
        return;
    }

    let Some(field) = fields.iter().next() else {
        return;
    };
    let text = field.text.as_str();
    if text.is_empty() {
        return;
    }

    let params = speech::SynthParams {
        speed: settings.get(Param::Speed) * BASE_SPEED_SCALE,
        pitch: settings.get(Param::Pitch),
        vibrato: settings.get(Param::Vibrato),
        brightness: settings.get(Param::Brightness),
    };
    let samples = speech::synth_babble(text, params);
    let seconds = samples.len() as f32 / (AUDIO_SAMPLE_RATE * AUDIO_CHANNELS_COUNT) as f32;
    let Some(clip) = AudioClip::new(samples.into(), &mut audio_state, None) else {
        return;
    };
    println!("babble: \"{text}\", {seconds:.2} s");

    let handle = clips.insert(clip);
    for voice in voices.iter_mut() {
        if voice.clip != handle {
            clips.remove(&voice.clip);
        }
        voice.clip = handle.clone();
        voice.playback_time = 0.0;
        voice.should_force_playback_time = true;
        voice.is_playing = true;
    }
}

// Scales the circle with the RMS of the samples the audio device played last.
fn pulse_circle(
    time: Res<TimeResource>,
    audio_state: Res<AudioStateResource>,
    mut smoothed_rms: Local<f32>,
    mut circles: WorldQuery<&mut LocalRectComponent, WithFilter<GreenCircle>>,
) {
    let samples = audio_state.last_samples();
    let rms = if samples.is_empty() {
        0.0
    } else {
        (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt()
    };

    let keep = PULSE_SMOOTHING.powf(time.delta_time * 60.0);
    *smoothed_rms = *smoothed_rms * keep + rms * (1.0 - keep);

    let size = CIRCLE_SIZE * (1.0 + CIRCLE_PULSE_GAIN * smoothed_rms.min(1.0)).min(CIRCLE_PULSE_MAX);
    for rect in circles.iter_mut() {
        rect.scale = Vec2::splat(UiVal::vh(size).into());
    }
}

// A drag starts on a bar and keeps moving that slider until the button is released, wherever the
// cursor goes. `GlobalRectComponent` is in window pixels, like the mouse position.
fn drag_sliders(
    input: Res<InputResource>,
    mut settings: ResMut<SynthSettings>,
    mut dragged: Local<Option<Param>>,
    mut bars: WorldQuery<(&GlobalRectComponent, &mut TextComponent, &SliderBar)>,
) {
    let [x, y] = input.mouse.position.map(|v| v as f32);
    let contains = |rect: &GlobalRectComponent| {
        let min = rect.center - rect.scale * 0.5;
        let max = rect.center + rect.scale * 0.5;
        x >= min.x && x <= max.x && y >= min.y && y <= max.y
    };

    if input.mouse.is_just_pressed(MouseButton::Left) {
        *dragged = bars.iter().find(|(rect, _, _)| contains(rect)).map(|(_, _, bar)| bar.0);
    }
    if !input.mouse.is_pressed(MouseButton::Left) {
        *dragged = None;
    }
    let Some(param) = *dragged else {
        return;
    };

    for (rect, text, bar) in bars.iter_mut() {
        if bar.0 != param {
            continue;
        }

        let left = rect.center.x - rect.scale.x * 0.5;
        let progress = if rect.scale.x > f32::EPSILON {
            ((x - left) / rect.scale.x).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let (min, max) = param.range();
        let value = min + (max - min) * progress;

        settings.0[param as usize] = value;
        text.text = param.bar_text(value).into();
    }
}

fn show_slider_values(settings: Res<SynthSettings>, mut readouts: WorldQuery<(&mut TextComponent, &SliderReadout)>) {
    for (text, readout) in readouts.iter_mut() {
        text.text = format!("{:.2}x", settings.get(readout.0)).into();
    }
}
