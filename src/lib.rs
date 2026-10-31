//! `sm64coopdx-rs` — a Rust rewrite of the core of **sm64coopdx**, the online
//! multiplayer mod of the Super Mario 64 PC port.
//!
//! This crate mirrors the upstream module layout under `src/`:
//! ```text
//! src/
//!   types.rs      fixed-width int/float types (from include/types.h)
//!   math/         Vec3 / Mat4 / trig (from include/types.h)
//!   game/         game logic:
//!     object.rs        the Object entity system (from include/types.h)
//!     object_list.rs   object pool + per-category lists (spawn_object.c)
//!     game_loop.rs     fixed-timestep loop (pc_main.c, game_init.c)
//!     mod.rs           owns the pool, drives per-frame updates
//!   engine/       level-script interpreter (planned)
//!   goddard/      graphics library (planned)
//!   audio/        audio engine (planned)
//!   pc/           SDL/PC layer (planned)
//!   netcode/      multiplayer netcode (planned)
//! ```
//!
//! The first subsystem ported end-to-end is the **game loop + entity system**:
//! the object pool, the per-category object lists, and the fixed-timestep loop
//! that updates every object. This is the architectural core of the game.
//!
//! Upstream repo: https://github.com/coop-deluxe/sm64coopdx

pub mod game;
pub mod math;
pub mod types;

/// Engine module (level-script interpreter). Planned for the next porting pass.
pub mod engine {
    pub mod level_script;
}

/// Graphics library (goddard). Planned for a later pass.
pub mod goddard {
    pub mod gfx;
}

/// Audio engine. Planned for a later pass.
pub mod audio {
    pub mod driver;
}

/// PC / SDL platform layer. Planned for a later pass.
pub mod pc {
    pub mod platform;
}

/// Multiplayer netcode — sm64coopdx's defining feature. Planned for a later pass.
pub mod netcode {
    pub mod network;
}
