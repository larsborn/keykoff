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
        cmd.creation_flags(CREATE_NEW_PROCESS_GROUP | DETACHED_PROCESS);
    }

    cmd.spawn()
        .map_err(|e| format!("Failed to launch '{}': {}", entry.executable, e))?;
    Ok(())
}
