//! # Process Spawner Module
//!
//! Provides platform-independent facilities for spawning, detaching, and
//! managing child processes (such as the REKTAL Kernel and GUI client).
//!
//! ## Process Independence & Detachment
//! All processes spawned via this module are detached from the parent Launcher
//! process lifecycle:
//! - On **Linux / macOS (Unix)**: Child processes and terminal emulators are placed in their own
//!   process group (`process_group(0)` / `setpgid`) and have standard streams decoupled, ensuring
//!   they outlive the Launcher when it closes.
//! - On **Windows**: Processes are detached using `CREATE_NEW_PROCESS_GROUP` and (optionally)
//!   `CREATE_NEW_CONSOLE`.

use std::process::{Command, Stdio};
use common::logging::LogLevel::Info;
use common::r_log;

/// Spawns a REKTAL binary (e.g. `"kernel"` or `"gui"`) as an independent, detached process.
///
/// Resolves the sibling binary located alongside the current launcher executable and launches
/// it with the provided CLI arguments.
///
/// # Arguments
///
/// * `bin_name` - The base name of the target binary to spawn (e.g. `"kernel"`, `"gui"`).
///   On Windows, `.exe` is automatically checked and appended if applicable.
/// * `args` - A slice of command-line argument strings to pass to the spawned process.
/// * `show_console` - When `true`, opens the process inside a visible, native OS terminal
///   window with a pause prompt on termination. When `false`, launches the process silently
///   in the background with suppressed standard streams.
///
/// # Returns
///
/// Returns a [`std::io::Result<std::process::Child>`] representing the spawned child process handle.
///
/// # Errors
///
/// Returns an [`std::io::Error`] if:
/// * The current executable path cannot be determined.
/// * The target binary cannot be found or executed.
/// * The platform terminal emulator cannot be spawned.
///
/// # Platform Behavior
///
/// * **Windows:** Utilizes `CREATE_NEW_CONSOLE` and `CREATE_NEW_PROCESS_GROUP` creation flags.
/// * **Linux:** Prioritizes `$TERMINAL`, `xdg-terminal-exec`, `x-terminal-emulator`, and fallback
///   terminal emulators (`gnome-terminal`, `konsole`, `xterm`). Sets `process_group(0)` to decouple.
/// * **macOS:** Dispatches script execution to `Terminal.app` via `osascript` or runs headless with `process_group(0)`.
pub fn spawn_process(bin_name: &str, args: &[&str], show_console: bool) -> std::io::Result<std::process::Child> {
    let exe_dir = std::env::current_exe()?
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| std::path::PathBuf::from("."));

    #[cfg(target_os = "windows")]
    let target_exe = if exe_dir.join(format!("{}.exe", bin_name)).exists() {
        exe_dir.join(format!("{}.exe", bin_name))
    } else {
        exe_dir.join(bin_name)
    };

    #[cfg(not(target_os = "windows"))]
    let target_exe = exe_dir.join(bin_name);

    #[cfg(target_os = "windows")]
    {
        spawn_windows(bin_name, &target_exe, &exe_dir, args, show_console)
    }
    #[cfg(target_os = "linux")]
    {
        spawn_linux(bin_name, &target_exe, &exe_dir, args, show_console)
    }
    #[cfg(target_os = "macos")]
    {
        spawn_macos(bin_name, &target_exe, &exe_dir, args, show_console)
    }
    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    {
        let mut cmd = Command::new(&target_exe);
        cmd.args(args).current_dir(&exe_dir);
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            cmd.process_group(0);
        }
        cmd.spawn()
    }
}

/// Spawns the REKTAL Kernel executable with optional arguments and console window.
///
/// Convenience wrapper around [`spawn_process`] with `bin_name = "kernel"`.
///
/// # Arguments
///
/// * `args` - Command-line arguments passed to the Kernel binary.
/// * `show_console` - Whether to display a native console window.
///
/// # Examples
///
/// ```no_run
/// use launcher::spawn_process::spawn_kernel;
///
/// // Spawns the kernel in a visible terminal window
/// let _child = spawn_kernel(&[], true);
/// ```
pub fn spawn_kernel(args: &[&str], show_console: bool) -> std::io::Result<std::process::Child> {
    spawn_process("rektal_kernel", args, show_console)
}

/// Spawns the REKTAL GUI client executable with optional arguments and console window.
///
/// Convenience wrapper around [`spawn_process`] with `bin_name = "gui"`.
///
/// # Arguments
///
/// * `args` - Command-line arguments passed to the GUI binary.
/// * `show_console` - Whether to display a native console window alongside the GUI.
///
/// # Examples
///
/// ```no_run
/// use launcher::spawn_process::spawn_gui;
///
/// // Spawns the GUI quietly in the background
/// let _child = spawn_gui(&[], false);
/// ```
pub fn spawn_gui(args: &[&str], show_console: bool) -> std::io::Result<std::process::Child> {
    spawn_process("rektal_gui", args, show_console)
}

/// Spawns a process on Windows with process group detachment and optional console window.
#[cfg(target_os = "windows")]
fn spawn_windows(
    _bin_name: &str,
    target_exe: &std::path::Path,
    cwd: &std::path::Path,
    args: &[&str],
    show_console: bool,
) -> std::io::Result<std::process::Child> {
    use std::os::windows::process::CommandExt;
    const CREATE_NEW_CONSOLE: u32 = 0x00000010;
    const CREATE_NEW_PROCESS_GROUP: u32 = 0x00000200;

    let mut cmd = Command::new(target_exe);
    cmd.args(args).current_dir(cwd);

    if show_console {
        cmd.creation_flags(CREATE_NEW_CONSOLE | CREATE_NEW_PROCESS_GROUP);
    } else {
        cmd.creation_flags(CREATE_NEW_PROCESS_GROUP);
    }

    cmd.spawn()
}

/// Spawns a process on Linux, querying available terminal emulators when console output is requested.
///
/// Decouples standard I/O and creates a separate process group (`process_group(0)`) so the process
/// outlives the Launcher.
#[cfg(target_os = "linux")]
fn spawn_linux(
    bin_name: &str,
    target_exe: &std::path::Path,
    cwd: &std::path::Path,
    args: &[&str],
    show_console: bool,
) -> std::io::Result<std::process::Child> {
    use std::os::unix::process::CommandExt;

    if !show_console {
        let mut cmd = Command::new(target_exe);
        cmd.args(args)
            .current_dir(cwd)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .process_group(0);
        return cmd.spawn();
    }

    let title = format!("REKTAL {}", bin_name.to_uppercase());
    let formatted_args = args
        .iter()
        .map(|a| format!("'{}'", a.replace('\'', "'\\''")))
        .collect::<Vec<_>>()
        .join(" ");

    let full_cmd = if formatted_args.is_empty() {
        format!(
            "cd '{}' && '{}'; echo ''; read -p '{} terminated. Press Enter to close...' -r",
            cwd.display(),
            target_exe.display(),
            bin_name
        )
    } else {
        format!(
            "cd '{}' && '{}' {}; echo ''; read -p '{} terminated. Press Enter to close...' -r",
            cwd.display(),
            target_exe.display(),
            formatted_args,
            bin_name
        )
    };

    let mut candidates = Vec::new();
    if let Ok(user_term) = std::env::var("TERMINAL") {
        candidates.push(user_term);
    }
    candidates.extend([
        "xdg-terminal-exec".to_string(),
        "x-terminal-emulator".to_string(),
        "gnome-terminal".to_string(),
        "konsole".to_string(),
        "xterm".to_string(),
    ]);

    for term in &candidates {
        let mut cmd = Command::new(term);
        if term == "xdg-terminal-exec" {
            cmd.args(["bash", "-c", &full_cmd]);
        } else if term == "gnome-terminal" {
            cmd.args(["--title", &title, "--", "bash", "-c", &full_cmd]);
        } else if term == "konsole" {
            cmd.args(["-p", &format!("tabtitle={}", title), "-e", "bash", "-c", &full_cmd]);
        } else {
            cmd.args(["-e", "bash", "-c", &full_cmd]);
        }

        cmd.stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .process_group(0);

        if let Ok(child) = cmd.spawn() {
            return Ok(child);
        }
    }

    r_log!(Info, "Could not find terminal-application! running command discretely!");
    let mut cmd = Command::new(target_exe);
    cmd.args(args)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .process_group(0);
    cmd.spawn()
}

/// Spawns a process on macOS, dispatching to `Terminal.app` via AppleScript when console output is requested.
#[cfg(target_os = "macos")]
fn spawn_macos(
    bin_name: &str,
    target_exe: &std::path::Path,
    cwd: &std::path::Path,
    args: &[&str],
    show_console: bool,
) -> std::io::Result<std::process::Child> {
    use std::os::unix::process::CommandExt;

    if !show_console {
        let mut cmd = Command::new(target_exe);
        cmd.args(args)
            .current_dir(cwd)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .process_group(0);
        return cmd.spawn();
    }

    let formatted_args = args
        .iter()
        .map(|a| format!("'{}'", a.replace('\'', "'\\''")))
        .collect::<Vec<_>>()
        .join(" ");

    let full_cmd = if formatted_args.is_empty() {
        format!(
            "cd '{}' && '{}'; echo ''; read -p '{} terminated. Press Enter to close...' -r",
            cwd.display(),
            target_exe.display(),
            bin_name
        )
    } else {
        format!(
            "cd '{}' && '{}' {}; echo ''; read -p '{} terminated. Press Enter to close...' -r",
            cwd.display(),
            target_exe.display(),
            formatted_args,
            bin_name
        )
    };

    let script = format!(
        "tell application \"Terminal\" to do script \"{}\"",
        full_cmd.replace('\\', "\\\\").replace('\"', "\\\"")
    );

    let mut cmd = Command::new("osascript");
    cmd.args(["-e", &script])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .process_group(0);
    cmd.spawn()
}