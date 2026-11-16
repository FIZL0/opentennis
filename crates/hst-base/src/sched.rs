//! P140: how Bevy spreads the frame's systems over threads. Most of the game's systems are tiny, so the compute pool
//! spent more time waking and parking its threads (futex calls, queue pops, contended locks) than running them.
//!
//! `HST_COMPUTE=N`: N compute-pool threads instead of one per core left over after the I/O and async pools.
//! `HST_EXECUTOR=single`: the main world's schedules run their systems one after another on the main thread;
//! `HST_EXECUTOR=single-all`: the render world's too; `HST_EXECUTOR=multi`: Bevy's multi-threaded executor everywhere.
//! Unset: the defaults below (what the P140 measurements picked: 4 compute threads, the multi-threaded executor).

use bevy::app::{App, Last, TaskPoolOptions, TaskPoolPlugin};
use bevy::ecs::intern::Interned;
use bevy::ecs::schedule::{ScheduleLabel, Schedules, SingleThreadedExecutor};
use bevy::prelude::*;
use bevy::render::RenderApp;
use std::collections::HashSet;

/// Which schedules run single-threaded.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Executor {
    /// Bevy's multi-threaded executor everywhere.
    Multi,
    /// The main world's schedules on one thread; the render world keeps its multi-threaded executor.
    Main,
    /// Main and render world both on one thread each.
    All,
}

/// The executor choice: `HST_EXECUTOR`, else the default.
pub fn executor() -> Executor {
    match std::env::var("HST_EXECUTOR").as_deref() {
        Ok("multi") => Executor::Multi,
        Ok("single") => Executor::Main,
        Ok("single-all") => Executor::All,
        _ => DEFAULT_EXECUTOR,
    }
}

/// What a run gets without `HST_EXECUTOR`: Bevy's. The single-threaded executor saved nothing over 4 compute threads
/// (P140), and alone it kept the 8-thread pool's wake-ups for the render world and transform propagation.
pub const DEFAULT_EXECUTOR: Executor = Executor::Multi;

/// The compute pool's thread cap: `HST_COMPUTE`, else the default (`None`: Bevy's, every core left over).
pub fn compute_threads() -> Option<usize> {
    std::env::var("HST_COMPUTE").ok().and_then(|v| v.parse().ok()).filter(|&n| n >= 1).or(DEFAULT_COMPUTE)
}

/// What a run gets without `HST_COMPUTE`: 4. Bevy gives the compute pool every core the I/O and async pools leave
/// (8 of a 16-thread desktop), and their wake-ups cost more than the parallel work saves: on a 16-thread desktop 4
/// threads took a third to half less CPU per frame with steadier frames (P140). An 8-thread phone or handheld already
/// had 4, so it is unchanged; 2 or 3 gave worse 1 % lows. `HST_COMPUTE=99` restores Bevy's split.
pub const DEFAULT_COMPUTE: Option<usize> = Some(4);

/// `TaskPoolPlugin` with the compute pool capped as `compute_threads` says.
pub fn task_pool() -> TaskPoolPlugin {
    let mut options = TaskPoolOptions::default();
    if let Some(n) = compute_threads() {
        options.compute.max_threads = n;
    }
    TaskPoolPlugin { task_pool_options: options }
}

/// Switches the schedules `executor` names to the single-threaded executor: the ones there now, and (a system in
/// `Last`) any a mode adds later. Call once the plugins are in.
pub fn apply(app: &mut App) {
    let kind = executor();
    if kind == Executor::Multi {
        return;
    }
    let mut done = HashSet::new();
    single(app.world_mut().resource_mut::<Schedules>().into_inner(), &mut done);
    app.add_systems(Last, move |mut schedules: ResMut<Schedules>, mut done: Local<HashSet<Interned<dyn ScheduleLabel>>>| {
        single(&mut schedules, &mut done);
    });
    if kind == Executor::All
        && let Some(render) = app.get_sub_app_mut(RenderApp)
    {
        let mut done = HashSet::new();
        single(render.world_mut().resource_mut::<Schedules>().into_inner(), &mut done);
        render.add_systems(bevy::render::Render, move |mut schedules: ResMut<Schedules>, mut done: Local<HashSet<Interned<dyn ScheduleLabel>>>| {
            single(&mut schedules, &mut done);
        });
    }
}

fn single(schedules: &mut Schedules, done: &mut HashSet<Interned<dyn ScheduleLabel>>) {
    for (_, schedule) in schedules.iter_mut() {
        if done.insert(schedule.label()) {
            schedule.set_executor(SingleThreadedExecutor::new());
        }
    }
}
