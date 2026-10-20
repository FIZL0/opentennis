//! The match's predicted path of the ball in play, which the CPU players copy from (`rally::Rally::copy_path`).
//!
//! A second ball, the predictor, is stepped with the live ball's physics (court plane only), 15 frames a game
//! frame, into a buffer of at most `CAP` entries. The path starts at a launch (`hit`) or a net touch (`net` +
//! `new_path`) and grows until its second court contact, then only tops up when the AI's window (it starts at
//! "frames since the path began") is about to run dry.

#![deny(clippy::float_arithmetic)]

use crate::ball::{Flight, Shot, Surface};
use crate::judge::Call;
use crate::ps2::F;
use crate::rally::Ball;

/// Entries the buffer holds.
pub const CAP: usize = 600;
/// Entries added per frame (the first build: the unstepped predictor and 14 steps).
const BATCH: usize = 15;
/// Court contacts after which the path only tops up.
const CONTACTS: i32 = 2;
/// Frames nobody looks at the ball after a swing (doubles).
const LOOK_FRAMES: i32 = 15;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Path {
    /// Entries written so far, y up.
    pub entries: Vec<Ball>,
    /// Frames since the path began: where the AI's window starts.
    pub start: i32,
    /// The path still grows.
    pub active: bool,
    /// The predictor's line call as of the last entry.
    pub mode: Call,
    /// The predictor's line distance when a prediction last turned out (kept across paths).
    pub gap: f32,
    /// Doubles: frames until players turn to look at the ball again (presentation only).
    pub look: i32,
    pub predictor: Flight,
}

impl Default for Path {
    /// No path yet; the predictor is seeded by the first launch.
    fn default() -> Self {
        Self::new(Flight::new(crate::ball::Ball { pos: [0.0; 3], vel: [0.0; 3], spin: 0.0 }, [[0.0; 4]; 4], [[0.0; 4]; 4]))
    }
}

impl Path {
    pub fn new(predictor: Flight) -> Self {
        Self { entries: Vec::with_capacity(CAP), start: 0, active: false, mode: Call::None, gap: 0.0, look: 0, predictor }
    }

    /// The window the AI copies from: entries from `start` on.
    pub fn window(&self) -> &[Ball] {
        self.entries.get(self.start.max(0) as usize..).unwrap_or(&[])
    }

    /// Forgets the path (inactive).
    pub fn stop(&mut self) {
        self.reset(false);
    }

    /// As `stop`, at a serve, a change of ends or a point start: nobody holds their look either.
    pub fn point(&mut self) {
        self.stop();
        self.look = 0;
    }

    /// A new path from the predictor as it stands, built on the next `frame`.
    pub fn new_path(&mut self) {
        self.reset(true);
    }

    /// A shot was launched: the predictor becomes the live ball `live` (just launched) and, from the serve phase on,
    /// the path is built now. `calls`: the predictor makes line calls (the rally phase).
    pub fn hit(&mut self, live: &Flight, shot: &Shot, surface: &Surface, calls: bool, serve_or_rally: bool) {
        self.predictor = *live;
        if serve_or_rally {
            self.reset(true);
            self.grow(shot, surface, calls);
        }
    }

    /// The live ball `live` touched the net (after its step): the predictor takes it over but keeps its own counts.
    /// The caller then sends NEW_PATH (`new_path`).
    pub fn net(&mut self, live: &Flight) {
        let p = &self.predictor;
        let (bounces, contacts, special) = (p.bounces, p.contacts, p.special_contacts);
        self.predictor = Flight { bounces, contacts, special_contacts: special, call: Call::None, ..*live };
    }

    /// The once-a-frame update. `swinging`: doubles, a player is in a swing (None in singles).
    pub fn frame(&mut self, shot: &Shot, surface: &Surface, calls: bool, swinging: Option<bool>) {
        if !self.entries.is_empty() {
            self.start += 1;
        }
        if self.active {
            self.grow(shot, surface, calls);
        }
        if let Some(swinging) = swinging {
            self.look = if swinging { LOOK_FRAMES } else { (self.look - 1).max(0) };
        }
    }

    fn reset(&mut self, active: bool) {
        self.entries.clear();
        (self.start, self.active, self.mode) = (0, active, Call::None);
    }

    fn grow(&mut self, shot: &Shot, surface: &Surface, calls: bool) {
        let mut steps = BATCH;
        match self.entries.last() {
            None => {
                self.mode = self.predictor.call;
                self.entries.push(self.entry());
                steps -= 1;
            }
            Some(last) if last.bounces >= CONTACTS && self.entries.len() as i32 - self.start >= BATCH as i32 => return,
            Some(_) => {}
        }
        let lines = self.predictor.lines;
        if !calls {
            self.predictor.lines = None;
        }
        self.predictor.net = false;
        for _ in 0..steps {
            self.predictor.step(shot, surface);
            if self.mode == Call::None && self.predictor.call == Call::Out {
                self.gap = self.predictor.line_distance;
            }
            self.mode = self.predictor.call;
            self.entries.push(self.entry());
        }
        self.predictor.lines = lines;
        if self.entries.len() >= CAP {
            self.active = false;
        }
    }

    fn entry(&self) -> Ball {
        let b = &self.predictor.ball;
        let up = |v: f32| (F(v) * F(-1.0)).0;
        Ball { pos: [b.pos[0], up(b.pos[1]), b.pos[2], 1.0], vel: [b.vel[0], up(b.vel[1]), b.vel[2], 0.0], bounces: self.predictor.contacts }
    }
}
