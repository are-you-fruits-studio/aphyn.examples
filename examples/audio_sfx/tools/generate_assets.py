"""Generates the sound clips of the audio_sfx example into assets/.

Sine tones as 16-bit PCM WAV, in the layouts the engine's loader accepts:
  blip.wav        48 kHz stereo, 0.15 s: a short decaying 880 Hz one-shot
  hum.wav         48 kHz stereo, 2.0 s: a steady 200 Hz tone, a whole number of periods, so it loops seamlessly
  mono.wav        44.1 kHz mono, 2.0 s: a decaying 600 Hz tone (mono to stereo, resampled to 48 kHz)
  mono_short.wav  44.1 kHz mono, 0.5 s: the same tone cut to a typical sound-effect length
  melody.wav      48 kHz stereo, 4.7 s: the first two phrases of "Twinkle Twinkle Little Star"
  stereo.wav      48 kHz stereo, 3.3 s: 440 Hz left only, then 660 Hz right only, then both together

Deterministic, standard library only. Run from anywhere: python3 examples/audio_sfx/tools/generate_assets.py
"""

import math
import os
import struct
import wave

OUT = os.path.normpath(os.path.join(os.path.dirname(__file__), "..", "assets"))


def write_wav(name: str, rate: int, channels: int, dur: float, freq: float, decay: float) -> None:
    os.makedirs(OUT, exist_ok=True)
    path = os.path.join(OUT, name)
    n = int(rate * dur)
    with wave.open(path, "w") as w:
        w.setnchannels(channels)
        w.setsampwidth(2)
        w.setframerate(rate)
        frames = bytearray()
        for i in range(n):
            t = i / rate
            env = math.exp(-decay * t) if decay > 0 else 1.0
            s = math.sin(2.0 * math.pi * freq * t) * env * 0.6
            s = max(-1.0, min(1.0, s))
            iv = int(s * 32767)
            for _ in range(channels):
                frames += struct.pack("<h", iv)
        w.writeframes(frames)
    print(f"wrote {path}  ({rate} Hz, {channels} ch, {dur}s, {freq} Hz)")


def write_melody(name: str, rate: int, notes: list, amp: float = 0.5) -> None:
    """Write a stereo tune. `notes` is a list of (freq_hz_or_None, dur_seconds);
    freq=None is a rest. Each note gets a short fade in/out to avoid clicks."""
    os.makedirs(OUT, exist_ok=True)
    path = os.path.join(OUT, name)
    fade = max(1, int(rate * 0.008))
    with wave.open(path, "w") as w:
        w.setnchannels(2)
        w.setsampwidth(2)
        w.setframerate(rate)
        frames = bytearray()
        for freq, dur in notes:
            n = int(rate * dur)
            for i in range(n):
                if freq is None:
                    s = 0.0
                else:
                    env = 1.0
                    if i < fade:
                        env = i / fade
                    elif i > n - fade:
                        env = max(0.0, (n - i) / fade)
                    s = math.sin(2.0 * math.pi * freq * (i / rate)) * env * amp
                iv = struct.pack("<h", int(max(-1.0, min(1.0, s)) * 32767))
                frames += iv + iv  # same sample to both channels
        w.writeframes(frames)
    total = sum(d for _, d in notes)
    print(f"wrote {path}  (melody, {rate} Hz, 2 ch, {total:.2f}s)")


def write_stereo(name: str, rate: int, segments: list, amp: float = 0.5) -> None:
    """Write a clip with INDEPENDENT left/right channels. `segments` is a list of
    (freq_left_or_None, freq_right_or_None, dur_seconds); None = silence on that
    channel, so a swapped or mono-summed output is easy to hear."""
    os.makedirs(OUT, exist_ok=True)
    path = os.path.join(OUT, name)
    fade = max(1, int(rate * 0.008))

    def sample_bytes(freq, i, n):
        if freq is None:
            return struct.pack("<h", 0)
        env = 1.0
        if i < fade:
            env = i / fade
        elif i > n - fade:
            env = max(0.0, (n - i) / fade)
        s = math.sin(2.0 * math.pi * freq * (i / rate)) * env * amp
        return struct.pack("<h", int(max(-1.0, min(1.0, s)) * 32767))

    with wave.open(path, "w") as w:
        w.setnchannels(2)
        w.setsampwidth(2)
        w.setframerate(rate)
        frames = bytearray()
        for fl, fr, dur in segments:
            n = int(rate * dur)
            for i in range(n):
                frames += sample_bytes(fl, i, n) + sample_bytes(fr, i, n)
        w.writeframes(frames)
    total = sum(d for _, _, d in segments)
    print(f"wrote {path}  (stereo L/R, {rate} Hz, 2 ch, {total:.2f}s)")


# Note frequencies (Hz)
C4, D4, E4, F4, G4, A4 = 261.63, 293.66, 329.63, 349.23, 392.00, 440.00

# "Twinkle Twinkle Little Star": first two phrases
TWINKLE = [
    (C4, 0.3), (C4, 0.3), (G4, 0.3), (G4, 0.3), (A4, 0.3), (A4, 0.3), (G4, 0.55),
    (F4, 0.3), (F4, 0.3), (E4, 0.3), (E4, 0.3), (D4, 0.3), (D4, 0.3), (C4, 0.55),
]

# LEFT-only (440 Hz) -> gap -> RIGHT-only (660 Hz) -> gap -> both (different tones)
STEREO = [
    (440.0, None, 1.0),
    (None, None, 0.15),
    (None, 660.0, 1.0),
    (None, None, 0.15),
    (440.0, 660.0, 1.0),
]

if __name__ == "__main__":
    write_wav("blip.wav", 48000, 2, 0.15, 880.0, 18.0)
    write_wav("hum.wav", 48000, 2, 2.00, 200.0, 0.0)
    write_wav("mono.wav", 44100, 1, 2.00, 600.0, 1.2)
    write_wav("mono_short.wav", 44100, 1, 0.50, 600.0, 1.2)
    write_melody("melody.wav", 48000, TWINKLE)
    write_stereo("stereo.wav", 48000, STEREO)
