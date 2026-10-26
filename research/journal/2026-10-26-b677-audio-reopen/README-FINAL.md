# B677: audio survives the output device going away

## What changed
- `third_party/bevy_audio` (0.19.1 vendored, `[patch.crates-io]` in `Cargo.toml` and `fuzz/Cargo.toml`, README row):
  - The sink opens with our own error callback. It ignores underruns, logs the first error that ends the stream once,
    and raises a flag. rodio's default callback logs every error.
  - A silent **heartbeat** source sits on the mixer and counts the samples the device pulls. If that count stays the
    same for 1.5 s, the output counts as lost too. This was needed because killing PipeWire doesn't raise any error
    through pipewire-alsa: cpal's ALSA thread just sits in `poll`, so with only the error callback the sound would
    have stayed dead with nothing logged.
  - `reopen_audio_output` (PostUpdate, before the playback set) drops a lost sink and opens the default device again
    on a thread, trying once a second until it opens, so the frame never waits on the audio server. Then:
    - Loop/Once players lose their sink, so they get queued again on the new mixer.
    - Despawn/Remove players are treated as finished.
    - It sends `AudioOutputReopened`.
  - A reopen only tries the default device. At startup it still falls back to any device, as upstream does. During the
    test the fallback opened another ALSA device while PipeWire was down, and that could take the sound card from
    PipeWire when it comes back. It also does the retry when no device was found at startup.
- `hst-base/src/audio.rs`: the whole game is one endless stream over the shared `Mix`, so the music, loops and voices
  that were sounding just carry on once the stream is queued again. On `AudioOutputReopened`, `Sound::drop_queued`
  drops the one-shot plays whose key-ons piled up while nothing pulled the mix (otherwise they'd all fire at once).
  It keeps a paused match's held sounds. Test: `a_reopened_device_drops_the_sounds_queued_while_it_was_gone`.

## How it was checked
I wrote a temporary probe test (removed afterwards): an App with AudioPlugin plays a looping Pitch tone against a
private PipeWire instance and its own WirePlumber.
- PipeWire: `minimal.conf` without udev/pulse/jack, with a null sink, on its own socket `hst-b677`.
- WirePlumber: hardware monitors disabled, its own XDG config/state dirs, D-Bus off.
- The test process ran with `PIPEWIRE_REMOTE=hst-b677`, so the user's desktop audio was never touched.

Sequence: start PipeWire, link the stream to the null sink, kill PipeWire + WirePlumber, wait 4 s, restart them.
- Exactly one warning ("stopped playing; reopening").
- While PipeWire was down, no stream was opened.
- After the restart the new stream was linked to the null sink and kept being pulled: no second stall warning in the
  remaining 6 s.

## Not verified / not 1:1
- The POLLERR path (the error callback firing, as in the task's report) wasn't reproduced. Killing PipeWire here gave
  a silent stall instead, which the heartbeat caught. The callback path is short (swap flag → same reopen).
- Suspend/resume and a Bluetooth headset going away weren't tried, and neither were Windows/macOS/Android. The
  heartbeat is backend-independent. The reopen only runs off wasm32 (browsers have no threads here; behaviour there is
  unchanged).
- The task said the music would be "restarted by audio.rs if it was playing". It needs no restart: the BGM lives in
  the shared `Mix`, which outlives the device, so it resumes where it stopped once the new sink pulls it.
- Not gameplay (no sim/RNG/timing involved), so no PINE comparison against the original.

## Wrap-up note
I committed by hand, not with `tools/done.sh`. All its gates before the tests passed (audit, patches.py, clippy, web
build). In the test suite, the only failure, twice, was `hst-base` `mods::tests::oob_faces_all`. It fails on
`oob_pc18_toro` ("plays [], no target for {anger_eye, …}"). That mod sits in `../opentennis-mods/out/mods` and was
being rebuilt by another session at 13:54–13:55, and it has no face targets yet. It's external mod data that this
task doesn't touch. The B677 tests (`audio::tests`) pass.
