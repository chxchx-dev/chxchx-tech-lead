//! One owner per job store. The lock file holds the owner's PID; a lock left behind by a process
//! that no longer exists is taken over.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

pub struct OwnerLock {
    path: PathBuf,
}

impl OwnerLock {
    pub fn acquire(path: &Path) -> Result<Self, String> {
        for _ in 0..2 {
            match OpenOptions::new().write(true).create_new(true).open(path) {
                Ok(mut file) => {
                    file.write_all(std::process::id().to_string().as_bytes())
                        .map_err(|error| {
                            format!("No se pudo escribir {}: {error}", path.display())
                        })?;
                    return Ok(Self {
                        path: path.to_owned(),
                    });
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                    let owner = fs::read_to_string(path)
                        .ok()
                        .and_then(|text| text.trim().parse::<u32>().ok());
                    if let Some(pid) = owner
                        && crate::features::workspace_processes::pid_alive(pid)
                    {
                        return Err(format!(
                            "Los trabajos ya los administra otro proceso (PID {pid})"
                        ));
                    }
                    // Stale: its owner is gone. Removing it and retrying `create_new` means only
                    // one of two processes racing for it can win.
                    let _ = fs::remove_file(path);
                }
                Err(error) => {
                    return Err(format!("No se pudo crear {}: {error}", path.display()));
                }
            }
        }
        Err(format!("No se pudo tomar el candado {}", path.display()))
    }
}

impl Drop for OwnerLock {
    fn drop(&mut self) {
        let mine = fs::read_to_string(&self.path)
            .is_ok_and(|text| text.trim() == std::process::id().to_string());
        if mine {
            let _ = fs::remove_file(&self.path);
        }
    }
}
