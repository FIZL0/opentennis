use crate::{
    AudioPlayer, Decodable, DefaultSpatialScale, GlobalVolume, PlaybackMode, PlaybackSettings,
    SpatialAudioSink, SpatialListener,
};
use bevy_asset::{Asset, Assets};
use bevy_ecs::{prelude::*, system::SystemParam};
use bevy_math::Vec3;
use bevy_transform::prelude::GlobalTransform;
use rodio::cpal::traits::{DeviceTrait, HostTrait};
use rodio::{DeviceSinkBuilder, MixerDeviceSink, Player, Source, SpatialPlayer};
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc,
};
#[cfg(not(target_arch = "wasm32"))]
use std::sync::Mutex;
use tracing::warn;

use crate::{AudioSink, AudioSinkPlayback};

/// Used internally to play audio on the current "audio device"
#[derive(Resource)]
pub(crate) struct AudioOutput {
    stream: Option<MixerDeviceSink>,
    // HST (B677): set by the open sink's error callback once its device has gone away (a PipeWire restart, a
    // suspend, a Bluetooth headset switched off), so `reopen_audio_output` drops it and opens the default sink again
    #[cfg(not(target_arch = "wasm32"))]
    lost: Arc<AtomicBool>,
    // HST (B677): how far the device has pulled the mixer, and when that last moved: an output whose server went
    // away can also just stop pulling with no error at all (pipewire-alsa sits in `poll` once PipeWire is killed)
    #[cfg(not(target_arch = "wasm32"))]
    pulled: (Arc<AtomicU64>, u64, std::time::Instant),
    // HST (B677): where the opener thread leaves the next sink; `None` while no opener runs
    #[cfg(not(target_arch = "wasm32"))]
    reopening: Option<Arc<Mutex<Option<Opened>>>>,
}

impl Default for AudioOutput {
    fn default() -> Self {
        let (stream, lost, pulls) = match open_default_sink(true) {
            Some(Opened { stream, lost, pulls }) => (Some(stream), lost, pulls),
            None => {
                warn!("No audio device found.");
                (None, Arc::default(), Arc::default())
            }
        };
        #[cfg(target_arch = "wasm32")]
        let _ = (lost, pulls);
        Self {
            stream,
            #[cfg(not(target_arch = "wasm32"))]
            lost,
            #[cfg(not(target_arch = "wasm32"))]
            pulled: (pulls, 0, std::time::Instant::now()),
            #[cfg(not(target_arch = "wasm32"))]
            reopening: None,
        }
    }
}

/// HST (B677): an open sink, the flag its error callback raises and the count of samples its device has pulled.
struct Opened {
    stream: MixerDeviceSink,
    lost: Arc<AtomicBool>,
    pulls: Arc<AtomicU64>,
}

/// HST (B677): a silent source on the mixer that counts the samples the device pulls through it, by the 1024.
struct Heartbeat {
    pulls: Arc<AtomicU64>,
    n: u32,
    channels: rodio::ChannelCount,
    rate: rodio::SampleRate,
}

impl Iterator for Heartbeat {
    type Item = rodio::Sample;
    fn next(&mut self) -> Option<rodio::Sample> {
        self.n += 1;
        if self.n == 1024 {
            self.n = 0;
            self.pulls.fetch_add(1, Ordering::Relaxed);
        }
        Some(0.0)
    }
}

impl Source for Heartbeat {
    fn current_span_len(&self) -> Option<usize> {
        None
    }
    fn channels(&self) -> rodio::ChannelCount {
        self.channels
    }
    fn sample_rate(&self) -> rodio::SampleRate {
        self.rate
    }
    fn total_duration(&self) -> Option<core::time::Duration> {
        None
    }
}

/// HST (B677): opens the default sink (else, with `fallback`, the first other device that opens: at start, as
/// upstream does; a reopen waits for the default instead, so it never takes a sound card from the audio server
/// coming back) with an error callback that logs
/// the first error which ends the stream and raises the returned flag, instead of rodio's default one, which logs
/// every error (thousands a second once ALSA's poll fails) and leaves the sound dead.
fn open_default_sink(fallback: bool) -> Option<Opened> {
    let lost = Arc::new(AtomicBool::new(false));
    let callback = {
        let lost = lost.clone();
        move |err: rodio::cpal::StreamError| {
            // an underrun is a glitch the stream plays on through
            if !matches!(err, rodio::cpal::StreamError::BufferUnderrun) && !lost.swap(true, Ordering::Relaxed) {
                warn!("Audio output lost ({err}); reopening the default device.");
            }
        }
    };
    let default = DeviceSinkBuilder::from_default_device()
        .and_then(|b| b.with_error_callback(callback.clone()).open_stream());
    let stream = default.ok().or_else(|| {
        if !fallback {
            return None;
        }
        rodio::cpal::default_host()
            .output_devices()
            .ok()?
            .filter(|dev| {
                dev.description()
                    .map(|desc| desc.driver().is_some_and(|driver| driver != "null"))
                    .unwrap_or(false)
            })
            .find_map(|d| {
                DeviceSinkBuilder::from_device(d)
                    .ok()?
                    .with_error_callback(callback.clone())
                    .open_sink_or_fallback()
                    .ok()
            })
    })?;
    let mut stream = stream;
    stream.log_on_drop(false);
    let pulls = Arc::new(AtomicU64::new(0));
    let config = stream.config();
    stream.mixer().add(Heartbeat {
        pulls: pulls.clone(),
        n: 0,
        channels: config.channel_count(),
        rate: config.sample_rate(),
    });
    Some(Opened { stream, lost, pulls })
}

/// Sent when the audio output has been opened again after its device went away (or after none was found at
/// start): the sounds that were playing have been queued again on the new device from their start (looping and
/// [`PlaybackMode::Once`] ones; despawn/remove ones are finished), so an endless stream should drop what piled up
/// while nothing was listening.
// HST (B677)
#[derive(Message, Debug, Clone, Copy, Default)]
pub struct AudioOutputReopened;

/// HST (B677): drops the sink once its device has gone away and opens the default one again on a thread (about
/// once a second until one opens, so the frame never waits on the audio server), then plays the sounds that were
/// playing on the new device's mixer.
pub(crate) fn reopen_audio_output(
    mut audio_output: ResMut<AudioOutput>,
    playing: Query<
        (Entity, &PlaybackSettings),
        Or<(With<AudioSink>, With<SpatialAudioSink>)>,
    >,
    mut reopened: MessageWriter<AudioOutputReopened>,
    mut commands: Commands,
) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let output = &mut *audio_output;
        if output.stream.is_some() {
            let (pulls, seen, at) = &mut output.pulled;
            let now = pulls.load(Ordering::Relaxed);
            if now != *seen {
                (*seen, *at) = (now, std::time::Instant::now());
            } else if at.elapsed() > core::time::Duration::from_millis(1500)
                && !output.lost.swap(true, Ordering::Relaxed)
            {
                warn!("Audio output stopped playing; reopening the default device.");
            }
            if output.lost.load(Ordering::Relaxed) {
                output.stream = None;
            }
        }
        if output.stream.is_some() {
            return;
        }
        let Some(slot) = output.reopening.clone() else {
            let slot = Arc::new(Mutex::new(None));
            output.reopening = Some(slot.clone());
            let opener = slot.clone();
            let started = std::thread::Builder::new()
                .name("audio reopen".into())
                .spawn(move || {
                    // the opener stops when the game has dropped its end of the slot
                    while Arc::strong_count(&opener) > 1 {
                        if let Some(sink) = open_default_sink(false) {
                            *opener.lock().unwrap() = Some(sink);
                            return;
                        }
                        std::thread::sleep(core::time::Duration::from_secs(1));
                    }
                });
            if started.is_err() {
                output.reopening = None;
            }
            return;
        };
        let Some(Opened { stream, lost, pulls }) = slot.lock().unwrap().take() else {
            return;
        };
        output.reopening = None;
        output.stream = Some(stream);
        output.lost = lost;
        output.pulled = (pulls, 0, std::time::Instant::now());
        for (entity, settings) in &playing {
            let mut entity = commands.entity(entity);
            match settings.mode {
                PlaybackMode::Loop | PlaybackMode::Once => {
                    entity.remove::<(AudioSink, SpatialAudioSink)>();
                }
                PlaybackMode::Despawn => {
                    entity.despawn();
                }
                PlaybackMode::Remove => {
                    entity.remove::<(AudioSink, SpatialAudioSink, PlaybackSettings, PlaybackRemoveMarker)>();
                }
            }
        }
        reopened.write(AudioOutputReopened);
    }
    #[cfg(target_arch = "wasm32")]
    let _ = (&mut audio_output, &playing, &mut reopened, &mut commands);
}

/// Marker for internal use, to despawn entities when playback finishes.
#[derive(Component, Default)]
pub struct PlaybackDespawnMarker;

/// Marker for internal use, to remove audio components when playback finishes.
#[derive(Component, Default)]
pub struct PlaybackRemoveMarker;

#[derive(SystemParam)]
pub(crate) struct EarPositions<'w, 's> {
    pub(crate) query: Query<'w, 's, (Entity, &'static GlobalTransform, &'static SpatialListener)>,
}

impl<'w, 's> EarPositions<'w, 's> {
    /// Gets a set of transformed ear positions.
    ///
    /// If there are no listeners, use the default values. If a user has added multiple
    /// listeners for whatever reason, we will return the first value.
    pub(crate) fn get(&self) -> (Vec3, Vec3) {
        let (left_ear, right_ear) = self
            .query
            .iter()
            .next()
            .map(|(_, transform, settings)| {
                (
                    transform.transform_point(settings.left_ear_offset),
                    transform.transform_point(settings.right_ear_offset),
                )
            })
            .unwrap_or_else(|| {
                let settings = SpatialListener::default();
                (settings.left_ear_offset, settings.right_ear_offset)
            });

        (left_ear, right_ear)
    }

    pub(crate) fn multiple_listeners(&self) -> bool {
        self.query.iter().len() > 1
    }
}

/// Plays "queued" audio through the [`AudioOutput`] resource.
///
/// "Queued" audio is any audio entity (with an [`AudioPlayer`] component) that does not have an
/// [`AudioSink`]/[`SpatialAudioSink`] component.
///
/// This system detects such entities, checks if their source asset
/// data is available, and creates/inserts the sink.
pub(crate) fn play_queued_audio_system<Source: Asset + Decodable>(
    audio_output: Res<AudioOutput>,
    audio_sources: Res<Assets<Source>>,
    global_volume: Res<GlobalVolume>,
    query_nonplaying: Query<
        (
            Entity,
            &AudioPlayer<Source>,
            &PlaybackSettings,
            Option<&GlobalTransform>,
        ),
        (Without<AudioSink>, Without<SpatialAudioSink>),
    >,
    ear_positions: EarPositions,
    default_spatial_scale: Res<DefaultSpatialScale>,
    mut commands: Commands,
) where
    f32: rodio::cpal::FromSample<rodio::Sample>,
{
    let Some(stream) = audio_output.stream.as_ref() else {
        // audio output unavailable; cannot play sound
        return;
    };
    let mixer = stream.mixer();

    for (entity, source_handle, settings, maybe_emitter_transform) in &query_nonplaying {
        let Some(audio_source) = audio_sources.get(&source_handle.0) else {
            continue;
        };
        // audio data is available (has loaded), begin playback and insert sink component
        if settings.spatial {
            let (left_ear, right_ear) = ear_positions.get();

            // We can only use one `SpatialListener`. If there are more than that, then
            // the user may have made a mistake.
            if ear_positions.multiple_listeners() {
                warn!(
                    "Multiple SpatialListeners found. Using {}.",
                    ear_positions.query.iter().next().unwrap().0
                );
            }

            let scale = settings.spatial_scale.unwrap_or(default_spatial_scale.0).0;

            let emitter_translation = if let Some(emitter_transform) = maybe_emitter_transform {
                (emitter_transform.translation() * scale).into()
            } else {
                warn!("Spatial AudioPlayer with no GlobalTransform component. Using zero.");
                Vec3::ZERO.into()
            };

            let sink = SpatialPlayer::connect_new(
                mixer,
                emitter_translation,
                (left_ear * scale).into(),
                (right_ear * scale).into(),
            );

            let decoder = audio_source.decoder();

            match settings.mode {
                PlaybackMode::Loop => match (settings.start_position, settings.duration) {
                    // custom start position and duration
                    (Some(start_position), Some(duration)) => sink.append(
                        decoder
                            .skip_duration(start_position)
                            .take_duration(duration)
                            .repeat_infinite(),
                    ),

                    // custom start position
                    (Some(start_position), None) => {
                        sink.append(decoder.skip_duration(start_position).repeat_infinite());
                    }

                    // custom duration
                    (None, Some(duration)) => {
                        sink.append(decoder.take_duration(duration).repeat_infinite());
                    }

                    // full clip
                    (None, None) => sink.append(decoder.repeat_infinite()),
                },
                PlaybackMode::Once | PlaybackMode::Despawn | PlaybackMode::Remove => {
                    match (settings.start_position, settings.duration) {
                        (Some(start_position), Some(duration)) => sink.append(
                            decoder
                                .skip_duration(start_position)
                                .take_duration(duration),
                        ),

                        (Some(start_position), None) => {
                            sink.append(decoder.skip_duration(start_position));
                        }

                        (None, Some(duration)) => sink.append(decoder.take_duration(duration)),

                        (None, None) => sink.append(decoder),
                    }
                }
            }

            let mut sink = SpatialAudioSink::new(sink);

            if settings.muted {
                sink.mute();
            }

            sink.set_speed(settings.speed);
            sink.set_volume(settings.volume * global_volume.volume);

            if settings.paused {
                sink.pause();
            }

            match settings.mode {
                PlaybackMode::Loop | PlaybackMode::Once => commands.entity(entity).insert(sink),
                PlaybackMode::Despawn => commands
                    .entity(entity)
                    // PERF: insert as bundle to reduce archetype moves
                    .insert((sink, PlaybackDespawnMarker)),
                PlaybackMode::Remove => commands
                    .entity(entity)
                    // PERF: insert as bundle to reduce archetype moves
                    .insert((sink, PlaybackRemoveMarker)),
            };
        } else {
            let sink = Player::connect_new(mixer);
            let decoder = audio_source.decoder();

            match settings.mode {
                PlaybackMode::Loop => match (settings.start_position, settings.duration) {
                    // custom start position and duration
                    (Some(start_position), Some(duration)) => sink.append(
                        decoder
                            .skip_duration(start_position)
                            .take_duration(duration)
                            .repeat_infinite(),
                    ),

                    // custom start position
                    (Some(start_position), None) => {
                        sink.append(decoder.skip_duration(start_position).repeat_infinite());
                    }

                    // custom duration
                    (None, Some(duration)) => {
                        sink.append(decoder.take_duration(duration).repeat_infinite());
                    }

                    // full clip
                    (None, None) => sink.append(decoder.repeat_infinite()),
                },
                PlaybackMode::Once | PlaybackMode::Despawn | PlaybackMode::Remove => {
                    match (settings.start_position, settings.duration) {
                        (Some(start_position), Some(duration)) => sink.append(
                            decoder
                                .skip_duration(start_position)
                                .take_duration(duration),
                        ),

                        (Some(start_position), None) => {
                            sink.append(decoder.skip_duration(start_position));
                        }

                        (None, Some(duration)) => sink.append(decoder.take_duration(duration)),

                        (None, None) => sink.append(decoder),
                    }
                }
            }

            let mut sink = AudioSink::new(sink);

            if settings.muted {
                sink.mute();
            }

            sink.set_speed(settings.speed);
            sink.set_volume(settings.volume * global_volume.volume);

            if settings.paused {
                sink.pause();
            }

            match settings.mode {
                PlaybackMode::Loop | PlaybackMode::Once => commands.entity(entity).insert(sink),
                PlaybackMode::Despawn => commands
                    .entity(entity)
                    // PERF: insert as bundle to reduce archetype moves
                    .insert((sink, PlaybackDespawnMarker)),
                PlaybackMode::Remove => commands
                    .entity(entity)
                    // PERF: insert as bundle to reduce archetype moves
                    .insert((sink, PlaybackRemoveMarker)),
            };
        }
    }
}

pub(crate) fn cleanup_finished_audio<T: Decodable + Asset>(
    mut commands: Commands,
    query_nonspatial_despawn: Query<
        (Entity, &AudioSink),
        (With<PlaybackDespawnMarker>, With<AudioPlayer<T>>),
    >,
    query_spatial_despawn: Query<
        (Entity, &SpatialAudioSink),
        (With<PlaybackDespawnMarker>, With<AudioPlayer<T>>),
    >,
    query_nonspatial_remove: Query<
        (Entity, &AudioSink),
        (With<PlaybackRemoveMarker>, With<AudioPlayer<T>>),
    >,
    query_spatial_remove: Query<
        (Entity, &SpatialAudioSink),
        (With<PlaybackRemoveMarker>, With<AudioPlayer<T>>),
    >,
) {
    for (entity, sink) in &query_nonspatial_despawn {
        if sink.sink.empty() {
            commands.entity(entity).despawn();
        }
    }
    for (entity, sink) in &query_spatial_despawn {
        if sink.sink.empty() {
            commands.entity(entity).despawn();
        }
    }
    for (entity, sink) in &query_nonspatial_remove {
        if sink.sink.empty() {
            commands.entity(entity).remove::<(
                AudioPlayer<T>,
                AudioSink,
                PlaybackSettings,
                PlaybackRemoveMarker,
            )>();
        }
    }
    for (entity, sink) in &query_spatial_remove {
        if sink.sink.empty() {
            commands.entity(entity).remove::<(
                AudioPlayer<T>,
                SpatialAudioSink,
                PlaybackSettings,
                PlaybackRemoveMarker,
            )>();
        }
    }
}

/// Run Condition to only play audio if the audio output is available
pub(crate) fn audio_output_available(audio_output: Res<AudioOutput>) -> bool {
    audio_output.stream.is_some()
}

/// Updates spatial audio sinks when emitter positions change.
pub(crate) fn update_emitter_positions(
    mut emitters: Query<
        (&GlobalTransform, &SpatialAudioSink, &PlaybackSettings),
        Or<(Changed<GlobalTransform>, Changed<PlaybackSettings>)>,
    >,
    default_spatial_scale: Res<DefaultSpatialScale>,
) {
    for (transform, sink, settings) in emitters.iter_mut() {
        let scale = settings.spatial_scale.unwrap_or(default_spatial_scale.0).0;

        let translation = transform.translation() * scale;
        sink.set_emitter_position(translation);
    }
}

/// Updates spatial audio sink ear positions when spatial listeners change.
pub(crate) fn update_listener_positions(
    mut emitters: Query<(&SpatialAudioSink, &PlaybackSettings)>,
    changed_listener: Query<
        (),
        (
            Or<(
                Changed<SpatialListener>,
                Changed<GlobalTransform>,
                Changed<PlaybackSettings>,
            )>,
            With<SpatialListener>,
        ),
    >,
    ear_positions: EarPositions,
    default_spatial_scale: Res<DefaultSpatialScale>,
) {
    if !default_spatial_scale.is_changed() && changed_listener.is_empty() {
        return;
    }

    let (left_ear, right_ear) = ear_positions.get();

    for (sink, settings) in emitters.iter_mut() {
        let scale = settings.spatial_scale.unwrap_or(default_spatial_scale.0).0;

        sink.set_ears_position(left_ear * scale, right_ear * scale);
    }
}
