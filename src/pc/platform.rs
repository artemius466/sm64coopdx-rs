//! PC / SDL platform layer — stub.
//!
//! Ported from `src/pc/`. The upstream PC layer owns the SDL window, keyboard /
//! mouse / gamepad input, and the graphics backend selection. Full port is a
//! later pass; here we model the input state and window config contracts.

/// A pressed key state. Mirrors the upstream keyboard scan-code table.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum Key {
    #[default]
    None,
    Space,
    Enter,
    Escape,
    Up,
    Down,
    Left,
    Right,
    A,
    B,
    C,
    Z,
    Shift,
}

/// Keyboard state for one frame. Mirrors `gControllerInputs` / the scan-code
/// bitmask the upstream reads each frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyboardState {
    keys: [bool; 256],
}

impl KeyboardState {
    pub fn new() -> Self {
        KeyboardState::default()
    }

    pub fn press(&mut self, code: u8) {
        self.keys[code as usize] = true;
    }

    pub fn release(&mut self, code: u8) {
        self.keys[code as usize] = false;
    }

    pub fn is_pressed(&self, code: u8) -> bool {
        self.keys[code as usize]
    }
}

impl Default for KeyboardState {
    fn default() -> Self {
        KeyboardState { keys: [false; 256] }
    }
}

/// Window configuration. Mirrors `configWindow` in `src/pc/configfile.h`.
#[derive(Debug, Clone)]
pub struct WindowConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub fullscreen: bool,
    pub vsync: bool,
}

impl Default for WindowConfig {
    fn default() -> Self {
        WindowConfig {
            title: "sm64coopdx-rs".into(),
            width: 640,
            height: 480,
            fullscreen: false,
            vsync: true,
        }
    }
}

/// The platform's per-frame poll of input + window events.
/// Mirrors `gfx_sdl_handle_events` + the input read in `game_loop_one_iteration`.
pub trait Platform {
    /// Poll window / input events. Returns true if a quit was requested.
    fn poll_events(&mut self) -> bool;
    /// Read controller / keyboard input for this frame.
    fn read_controller_inputs(&mut self) -> u32;
    /// Request a graceful exit.
    fn request_quit(&mut self);
}
