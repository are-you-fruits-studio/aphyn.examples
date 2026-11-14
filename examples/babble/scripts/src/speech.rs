//! A tiny deterministic "babble" synthesizer: gibberish speech, not text-to-speech.
//!
//! Every non-space character becomes one short enveloped tone (a "syllable") whose pitch and length
//! come from the character's code point, and every whitespace character becomes a short pause. The
//! tone is a fundamental plus two overtones with a slow vibrato. Nothing is random, so the same text
//! and settings always give the same samples.

use std::f32::consts::TAU;

use aphyn::AUDIO_SAMPLE_RATE;

const SAMPLE_RATE: f32 = AUDIO_SAMPLE_RATE as f32;
const SPACE_GAP_SECS: f32 = 0.07;
const VIBRATO_RATE_HZ: f32 = 5.5;
// peak pitch deviation at `vibrato == 1.0`: +-3 %
const VIBRATO_DEPTH: f32 = 0.03;
const PEAK: f32 = 0.8;

// Each field multiplies one aspect of the sound; 1.0 everywhere is the plain voice.
#[derive(Clone, Copy)]
pub struct SynthParams {
    // shorter syllables and pauses at higher values, pitch unchanged
    pub speed: f32,
    // 2.0 is an octave up
    pub pitch: f32,
    // depth of the pitch wobble; 0.0 is a flat, robotic tone
    pub vibrato: f32,
    // loudness of the two overtones; 0.0 leaves a plain sine
    pub brightness: f32,
}

// Returns interleaved stereo samples at 48 kHz, ready for `AudioClip::new`.
pub fn synth_babble(text: &str, params: SynthParams) -> Vec<f32> {
    // a zero speed would make an endless clip
    let speed = params.speed.max(0.05);
    let mut mono = Vec::new();

    for c in text.chars() {
        if c.is_whitespace() {
            let gap = ((SPACE_GAP_SECS / speed) * SAMPLE_RATE) as usize;
            mono.extend(std::iter::repeat_n(0.0, gap));
        } else {
            synth_syllable(c, speed, params, &mut mono);
        }
    }

    normalize(&mut mono, PEAK);

    mono.iter().flat_map(|&s| [s, s]).collect()
}

fn synth_syllable(c: char, speed: f32, params: SynthParams, out: &mut Vec<f32>) {
    let code = c as u32;

    // 110-342 Hz and 60-132 ms, picked by the character code
    let base_freq = (110.0 + (code % 30) as f32 * 8.0) * params.pitch.max(0.0);
    let duration = (0.06 + (code % 5) as f32 * 0.018) / speed;

    let n = (duration * SAMPLE_RATE).max(1.0) as usize;
    let attack_n = ((n as f32) * 0.1).max(1.0) as usize;
    let overtones = params.brightness.max(0.0);

    for i in 0..n {
        let t = i as f32 / SAMPLE_RATE;

        let vibrato = 1.0 + VIBRATO_DEPTH * params.vibrato.max(0.0) * (t * VIBRATO_RATE_HZ * TAU).sin();
        let freq = base_freq * vibrato;

        let tone = (t * freq * TAU).sin() * 1.0
            + (t * freq * 2.0 * TAU).sin() * 0.5 * overtones
            + (t * freq * 2.7 * TAU).sin() * 0.25 * overtones;
        let tone = tone / 1.75;

        // fast linear attack, linear decay to zero, so syllables do not click at their edges
        let envelope = if i < attack_n {
            i as f32 / attack_n as f32
        } else {
            let decay = (i - attack_n) as f32 / (n - attack_n).max(1) as f32;
            (1.0 - decay).max(0.0)
        };

        out.push(tone * envelope);
    }
}

// Scales the buffer so its loudest sample is `target_peak`; leaves silence alone.
fn normalize(samples: &mut [f32], target_peak: f32) {
    let peak = samples.iter().fold(0.0f32, |m, s| m.max(s.abs()));
    if peak <= f32::EPSILON {
        return;
    }
    let scale = target_peak / peak;
    for s in samples.iter_mut() {
        *s *= scale;
    }
}
