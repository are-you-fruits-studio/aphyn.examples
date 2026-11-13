"""Generates assets/ambient_loop.wav: a calm 20 s ambient loop for the audio_screen_saver example.

Four soft pad chords (Cmaj9, Am9, Fmaj9, Gsus2), 5 s each, made of slightly detuned sine and
triangle voices spread across the stereo field, over a gentle low pulse at 60 BPM. Every note's
attack and release wrap around the end of the buffer, so the clip loops without a seam.
48 kHz, 16-bit stereo, peak at -6 dBFS. Deterministic (fixed seed).

Needs numpy. Run from anywhere: python3 examples/audio_screen_saver/tools/generate_clip.py
"""

import wave
from pathlib import Path

import numpy as np

SAMPLE_RATE = 48_000
CHORD_SECONDS = 5.0
PULSE_HZ = 1.0
PEAK = 0.5
SEED = 7

# MIDI notes: pad voicing and bass root of each chord
CHORDS = [
    ([60, 64, 67, 71, 74], 36),  # Cmaj9
    ([57, 60, 64, 67, 71], 33),  # Am9
    ([53, 57, 60, 64, 67], 29),  # Fmaj9
    ([55, 62, 67, 69, 74], 31),  # Gsus2
]

OUTPUT = Path(__file__).resolve().parent.parent / "assets" / "ambient_loop.wav"


def midi_to_hz(note):
    return 440.0 * 2.0 ** ((note - 69) / 12)


def triangle(phase):
    # band-limited: the first three odd harmonics keep it soft
    return sum(((-1) ** k) * np.sin((2 * k + 1) * phase) / (2 * k + 1) ** 2 for k in range(3)) * 8 / np.pi**2


def envelope(t, length, attack, release):
    # raised-cosine attack and release around a flat sustain
    env = np.ones_like(t)
    rising = t < attack
    env[rising] = 0.5 - 0.5 * np.cos(np.pi * t[rising] / attack)
    falling = t > length - release
    env[falling] = 0.5 + 0.5 * np.cos(np.pi * (t[falling] - (length - release)) / release)
    return env


def add_wrapped(buffer, start, signal):
    # writes past the end continue at the start, so the loop point is seamless
    indices = (start + np.arange(signal.shape[0])) % buffer.shape[0]
    np.add.at(buffer, indices, signal)


def main():
    rng = np.random.default_rng(SEED)
    chord_len = int(CHORD_SECONDS * SAMPLE_RATE)
    total = chord_len * len(CHORDS)
    out = np.zeros((total, 2))

    # each pad note starts 1.5 s before its chord and fades out 2.5 s after it, overlapping the neighbours
    lead_in, tail = 1.5, 2.5
    note_seconds = lead_in + CHORD_SECONDS + tail
    t = np.arange(int(note_seconds * SAMPLE_RATE)) / SAMPLE_RATE
    pad_env = envelope(t, note_seconds, 3.0, 4.0)

    for index, (notes, bass) in enumerate(CHORDS):
        start = index * chord_len - int(lead_in * SAMPLE_RATE)

        for note in notes:
            freq = midi_to_hz(note)
            pan = rng.uniform(-0.6, 0.6)
            voice = np.zeros_like(t)
            for detune, shape, gain in [(-0.15, np.sin, 0.5), (0.15, np.sin, 0.5), (0.0, triangle, 0.35)]:
                phase = 2 * np.pi * (freq + detune) * t + rng.uniform(0, 2 * np.pi)
                voice += gain * shape(phase)
            # a slow swell per note keeps the pad moving
            swell = 0.85 + 0.15 * np.sin(2 * np.pi * rng.uniform(0.1, 0.25) * t + rng.uniform(0, 2 * np.pi))
            voice *= pad_env * swell / len(notes)
            add_wrapped(out, start, np.stack([voice * (1 - pan) / 2, voice * (1 + pan) / 2], axis=1))

        # low pulse: the bass root and its octave, swelling once per beat
        bass_t = np.arange(chord_len + int(SAMPLE_RATE)) / SAMPLE_RATE
        freq = midi_to_hz(bass)
        tone = np.sin(2 * np.pi * freq * bass_t) + 0.3 * np.sin(4 * np.pi * freq * bass_t)
        beat = (bass_t * PULSE_HZ) % 1.0
        pulse = np.exp(-4.0 * beat) * (1 - np.exp(-60.0 * beat))
        bass_env = envelope(bass_t, bass_t[-1] + 1 / SAMPLE_RATE, 0.5, 1.0)
        low = 0.35 * tone * pulse * bass_env
        add_wrapped(out, index * chord_len, np.stack([low, low], axis=1))

    out *= PEAK / np.max(np.abs(out))
    pcm = np.round(out * 32767).astype("<i2")

    OUTPUT.parent.mkdir(parents=True, exist_ok=True)
    with wave.open(str(OUTPUT), "wb") as wav:
        wav.setnchannels(2)
        wav.setsampwidth(2)
        wav.setframerate(SAMPLE_RATE)
        wav.writeframes(pcm.tobytes())

    print(f"wrote {OUTPUT} ({total / SAMPLE_RATE:.1f} s, {OUTPUT.stat().st_size / 1e6:.2f} MB)")


if __name__ == "__main__":
    main()
