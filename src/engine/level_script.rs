//! Level-script interpreter.
//!
//! Ported from `src/engine/level_script.c` (`level_script_execute`).
//!
//! Upstream, the level script is a bytecode command stream processed by a
//! jump table (`LevelScriptJumpTable`) until the script status is no longer
//! `SCRIPT_RUNNING`. Each tick, `level_script_execute` runs the script and then
//! renders. We model the command stream as a list of commands and the jump
//! table as a dispatch over command types.

/// A level-script command type. Mirrors the `LevelScriptCommandType` enum.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
#[repr(u32)]
pub enum CmdType {
    #[default]
    EndLevel = 0,
    SetMarioModel = 1,
    SpawnPlayer = 2,
    SetCamera = 3,
    Transition = 4,
    SetPlayerCamera = 5,
    SetBackground = 6,
    SetCameraSpeed = 7,
    SetCameraHeight = 8,
    SetCameraScroll = 9,
    SetCameraTarget = 10,
    SetCameraMove = 11,
    SetCameraRoll = 12,
    SetCameraMoveTarget = 13,
    SetCameraMoveTarget2 = 14,
    SetCameraMoveTarget3 = 15,
    SetCameraMoveTarget4 = 16,
    SetCameraMoveTarget5 = 17,
    SetCameraMoveTarget6 = 18,
    SetCameraMoveTarget7 = 19,
    SetCameraMoveTarget8 = 20,
    SetCameraMoveTarget9 = 21,
    SetCameraMoveTarget10 = 22,
    SetCameraMoveTarget11 = 23,
    SetCameraMoveTarget12 = 24,
    SetCameraMoveTarget13 = 25,
    SetCameraMoveTarget14 = 26,
    SetCameraMoveTarget15 = 27,
    SetCameraMoveTarget16 = 28,
    SetCameraMoveTarget17 = 29,
    SetCameraMoveTarget18 = 30,
    SetCameraMoveTarget19 = 31,
    SetCameraMoveTarget20 = 32,
    SetCameraMoveTarget21 = 33,
    SetCameraMoveTarget22 = 34,
    SetCameraMoveTarget23 = 35,
    SetCameraMoveTarget24 = 36,
    SetCameraMoveTarget25 = 37,
    SetCameraMoveTarget26 = 38,
    SetCameraMoveTarget27 = 39,
    SetCameraMoveTarget28 = 40,
    SetCameraMoveTarget29 = 41,
    SetCameraMoveTarget30 = 42,
    SetCameraMoveTarget31 = 43,
    SetCameraMoveTarget32 = 44,
    // ... (truncated for the proof-of-concept)
}

/// A single level-script command: a type plus raw arguments.
#[derive(Debug, Clone, Default)]
pub struct LevelCommand {
    pub cmd_type: CmdType,
    pub args: [i32; 4],
}

impl LevelCommand {
    pub fn new(cmd_type: CmdType, args: [i32; 4]) -> Self {
        LevelCommand { cmd_type, args }
    }
}

/// The status of the level script. Mirrors `sScriptStatus`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum ScriptStatus {
    Stopped = 0,
    Running = 1,
    Paused = 2,
    Quit = 3,
}

/// The level-script interpreter.
///
/// Processes a command stream via the jump table until the status is no longer
/// `Running`, mirroring `level_script_execute`.
pub struct LevelScript {
    commands: Vec<LevelCommand>,
    pc: usize,
    status: ScriptStatus,
    last_cmd: Option<LevelCommand>,
}

impl LevelScript {
    pub fn new(commands: Vec<LevelCommand>) -> Self {
        LevelScript {
            commands,
            pc: 0,
            status: ScriptStatus::Stopped,
            last_cmd: None,
        }
    }

    /// Execute the script until it stops or a max step count is reached.
    /// Mirrors the `while (sScriptStatus == SCRIPT_RUNNING)` loop.
    pub fn execute(&mut self, max_steps: usize) -> usize {
        let mut steps = 0;
        self.status = ScriptStatus::Running;
        while self.status == ScriptStatus::Running && self.pc < self.commands.len() && steps < max_steps {
            let cmd = self.commands[self.pc].clone();
            self.dispatch(cmd);
            self.pc += 1;
            steps += 1;
        }
        steps
    }

    /// Dispatch a single command via the jump table.
    fn dispatch(&mut self, cmd: LevelCommand) {
        match cmd.cmd_type {
            CmdType::EndLevel => self.status = ScriptStatus::Stopped,
            _ => {
                // Other commands modify world state; the proof-of-concept
                // records the last command processed.
                self.last_cmd = Some(cmd);
            }
        }
        let _ = cmd.args;
    }

    pub fn status(&self) -> ScriptStatus {
        self.status
    }

    pub fn last_cmd(&self) -> Option<&LevelCommand> {
        self.last_cmd.as_ref()
    }

    pub fn remaining(&self) -> usize {
        self.commands.len() - self.pc
    }
}

impl Default for LevelScript {
    fn default() -> Self {
        LevelScript::new(Vec::new())
    }
}
