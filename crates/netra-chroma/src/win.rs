//! Apply `CREATE_NO_WINDOW` to child processes on Windows so the GUI Tauri
//! shell does not flash a console window when spawning helpers. No-op on
//! other platforms.

pub fn no_window(cmd: &mut tokio::process::Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    #[cfg(not(windows))]
    let _ = cmd;
}
