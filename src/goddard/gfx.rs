//! Graphics library (goddard) — stub.
//!
//! Ported from `src/goddard/` and `src/engine/`. The upstream goddard is the
//! graphics command library (GSPC/GSPWK) that builds display lists consumed by
//! the RDP. Full port is a later pass; here we model the display-list buffer
//! that the renderer fills each frame.

/// A display-list command, mirroring `Gfx` (the RDP command stream).
#[derive(Debug, Clone, Copy, Default)]
pub struct Gfx {
    pub op: u16,
    pub param: u32,
}

/// A minimal display-list buffer.
///
/// Mirrors the upstream `gDisplayListHead` / gfx pool: a fixed-capacity ring of
/// `Gfx` commands that the renderer fills each frame.
#[derive(Debug)]
pub struct DisplayList {
    commands: Vec<Gfx>,
}

impl DisplayList {
    pub fn new(capacity: usize) -> Self {
        DisplayList {
            commands: Vec::with_capacity(capacity),
        }
    }

    /// Append a command. Mirrors `gfx_append` / `dl_append`.
    pub fn push(&mut self, cmd: Gfx) {
        self.commands.push(cmd);
    }

    /// Number of commands queued this frame.
    pub fn len(&self) -> usize {
        self.commands.len()
    }

    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    pub fn commands(&self) -> &[Gfx] {
        &self.commands
    }

    /// Reset the buffer for the next frame.
    pub fn clear(&mut self) {
        self.commands.clear();
    }
}

impl Default for DisplayList {
    fn default() -> Self {
        DisplayList::new(1024)
    }
}
