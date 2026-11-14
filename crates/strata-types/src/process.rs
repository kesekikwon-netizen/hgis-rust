//! Helpers for running child console programs without a visible console window (`WIN1`).

use std::process::Command;

/// Win32 `CREATE_NO_WINDOW` process creation flag (`0x0800_0000`, `WinBase.h`).
pub const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Configures `cmd` so a console child process starts without a visible console window.
pub fn no_window(cmd: &mut Command) -> &mut Command {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

#[cfg(test)]
#[path = "process_tests.rs"]
mod tests;
