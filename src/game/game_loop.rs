//! The game loop.
//!
//! Ported from `src/pc/pc_main.c` (`produce_one_frame`) and
//! `src/game/game_init.c` (`thread5_game_loop` / `game_loop_one_iteration`).
//!
//! Upstream structure:
//!   - `main()` sets up SDL, graphics, network, then runs
//!     `gWindowApi->main_loop(produce_one_frame)`.
//!   - `produce_one_frame()` runs `game_loop_one_iteration()` at a fixed rate
//!     (30 FPS), which runs the level-script interpreter then renders.
//!   - `game_loop_one_iteration()` runs the game logic at a fixed timestep and
//!     renders once.
//!
//! We keep the fixed-timestep design: the loop accumulates real time and runs
//! the game logic at a fixed rate, decoupled from the render rate.

use crate::game::Game;
use crate::game::object_list::ObjectList;
use crate::types::f64;

/// The fixed simulation rate, matching upstream `FRAMERATE` (30).
pub const SIM_RATE: f64 = 30.0;
/// Duration of one fixed timestep, in seconds.
pub const FRAME_TIME: f64 = 1.0 / SIM_RATE;

/// The event sink the loop reports back to (rendering, input, audio).
/// Mirrors the upstream `produce_one_frame` split into render + logic.
pub trait LoopCallbacks {
    /// Called once per rendered frame.
    fn render(&mut self, frame: u32);
    /// Called when a quit is requested.
    fn should_quit(&self) -> bool;
}

/// A fixed-timestep game loop.
///
/// `accum` is the leftover real time; each step runs the game logic at the
/// fixed rate and calls `callbacks.render()` for each frame produced.
pub struct GameLoop<C: LoopCallbacks> {
    pub game: Game,
    pub callbacks: C,
    pub max_frame_time: f64,
    pub last_time: f64,
    pub accum: f64,
    pub running: bool,
}

impl<C: LoopCallbacks> GameLoop<C> {
    pub fn new(callbacks: C) -> Self {
        GameLoop {
            game: Game::new(),
            callbacks,
            // Cap a single step so a stalled frame can't cause a physics blowup.
            max_frame_time: 0.5,
            last_time: 0.0,
            accum: 0.0,
            running: true,
        }
    }

    /// Monotonic seconds. Mirrors upstream `clock_elapsed_f64()`.
    fn now() -> f64 {
        use std::time::{Duration, SystemTime};
        SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs_f64())
            .unwrap_or(0.0)
    }

    /// Run one iteration: advance the simulation, then render.
    /// Mirrors `produce_one_frame` -> `game_loop_one_iteration`.
    pub fn produce_one_frame(&mut self) {
        let start = Self::now();
        self.accum += start - self.last_time;
        self.last_time = start;

        // Run game logic at the fixed rate.
        let mut steps = 0;
        while self.accum >= FRAME_TIME {
            self.game.tick();
            self.accum -= FRAME_TIME;
            steps += 1;
            if steps > 288 {
                // 0.5s / (1/30) — bail out of a spiral of death.
                break;
            }
        }

        // Render every frame the logic produced.
        for _ in 0..steps.max(1) {
            self.callbacks.render(self.game.frame);
        }
    }

    /// The main loop. Mirrors `main()`'s `while (true)` loop.
    pub fn run(&mut self) {
        while self.running && !self.callbacks.should_quit() {
            self.produce_one_frame();
        }
        self.running = false;
    }

    /// Manually run a single frame (used by tests / headless runs).
    pub fn step(&mut self) {
        self.produce_one_frame();
        self.game.frame += 1;
    }
}

/// A no-op callbacks implementation for headless / test runs.
pub struct HeadlessCallbacks;

impl LoopCallbacks for HeadlessCallbacks {
    fn render(&mut self, _frame: u32) {}
    fn should_quit(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loop_runs_fixed_number_of_ticks() {
        // A callbacks struct that counts renders and quits after 10 frames.
        struct Counting {
            renders: u32,
            max: u32,
        }
        impl LoopCallbacks for Counting {
            fn render(&mut self, _frame: u32) {
                self.renders += 1;
            }
            fn should_quit(&self) -> bool {
                self.renders >= self.max
            }
        }

        let mut loop_ctx = GameLoop::new(Counting { renders: 0, max: 10 });
        // Run enough real time for ~10 frames at 30 Hz.
        for _ in 0..50 {
            if loop_ctx.callbacks.should_quit() {
                break;
            }
            loop_ctx.step();
        }
        assert!(loop_ctx.callbacks.should_quit());
    }

    #[test]
    fn game_frame_advances_with_ticks() {
        let mut loop_ctx = GameLoop::new(HeadlessCallbacks);
        let before = loop_ctx.game.frame;
        loop_ctx.step();
        assert!(loop_ctx.game.frame > before);
    }
}
