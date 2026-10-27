use crate::config::RunConfig;
use std::process::Command;

pub fn launch(entry: &RunConfig) -> Result<(), String> {
    let mut cmd = Command::new(&entry.executable);

    if !entry.parameters.is_empty() {
        let args: Vec<&str> = entry.parameters.split_whitespace().collect();
        cmd.args(&args);
    }

    if !entry.working_directory.is_empty() {
        let wd = std::path::Path::new(&entry.working_directory);
        if !wd.exists() {
            return Err(format!(
                "Working directory does not exist: {}",
                entry.working_directory
            ));
        }
        cmd.current_dir(wd);
    }

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x00000200;
        const DETACHED_PROCESS: u32 = 0x00000008;
        const CREATE_NEW_CONSOLE: u32 = 0x00000010;
        // Keykoff has no console of its own. A detached child gets none
        // either, which is right for GUI programs but makes console programs
        // (file managers, shells, anything built for a terminal) exit at once
        // because their standard handles are missing. Those get a console
        // window of their own instead.
        let console = if is_console_program(&entry.executable) {
            CREATE_NEW_CONSOLE
        } else {
            DETACHED_PROCESS
        };
        cmd.creation_flags(CREATE_NEW_PROCESS_GROUP | console);
    }

    cmd.spawn()
        .map_err(|e| format!("Failed to launch '{}': {}", entry.executable, e))?;
    Ok(())
}

/// Reads the PE header to tell console-subsystem executables (subsystem 3)
/// from GUI ones. Anything unreadable or not a PE file counts as GUI, which
/// keeps the previous behaviour.
pub fn is_console_program(path: &str) -> bool {
    const IMAGE_SUBSYSTEM_WINDOWS_CUI: u16 = 3;
    let data = match std::fs::read(path) {
        Ok(d) => d,
        Err(_) => return false,
    };
    if data.len() < 0x40 || &data[0..2] != b"MZ" {
        return false;
    }
    let pe = u32::from_le_bytes([data[0x3c], data[0x3d], data[0x3e], data[0x3f]]) as usize;
    // "PE\0\0", 20-byte COFF header, then the optional header; Subsystem sits
    // at offset 68 of the optional header for both PE32 and PE32+.
    let subsystem_at = pe + 4 + 20 + 68;
    if data.len() < subsystem_at + 2 || &data[pe..pe + 4] != b"PE\0\0" {
        return false;
    }
    u16::from_le_bytes([data[subsystem_at], data[subsystem_at + 1]]) == IMAGE_SUBSYSTEM_WINDOWS_CUI
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binary_is_a_console_program() {
        // The test harness itself is a console-subsystem executable.
        let me = std::env::current_exe().unwrap();
        assert!(is_console_program(me.to_str().unwrap()));
    }

    #[test]
    fn missing_or_non_pe_files_count_as_gui() {
        assert!(!is_console_program(r"C:\definitely\not\there.exe"));
        let dir = std::env::temp_dir().join("keykoff-launcher-test");
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("not-a-pe.exe");
        std::fs::write(&p, b"hello").unwrap();
        assert!(!is_console_program(p.to_str().unwrap()));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
