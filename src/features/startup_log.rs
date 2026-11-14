//! Startup failure log of Studio.
//!
//! On Windows Studio has no console (`windows_subsystem = "windows"`), so an error printed to
//! stderr is lost and a launch that fails looks like "nothing happens". Failures and panics are
//! appended to `<application home>/logs/studio-startup.log` so there is always something to read.
use super::project_profiles::application_home;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

const MAX_BYTES: u64 = 256 * 1024;

pub fn log_path() -> Option<PathBuf> {
    Some(application_home()?.join("logs").join("studio-startup.log"))
}

/// Appends one entry and returns where it went (`None` when no log could be written).
pub fn record(message: &str) -> Option<PathBuf> {
    let path = log_path()?;
    fs::create_dir_all(path.parent()?).ok()?;
    // A log that keeps growing is a problem of its own: start again past the limit.
    if fs::metadata(&path).is_ok_and(|meta| meta.len() > MAX_BYTES) {
        let _ = fs::remove_file(&path);
    }
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs());
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .ok()?;
    writeln!(
        file,
        "[{seconds}] chxchx-studio {}: {}",
        env!("CARGO_PKG_VERSION"),
        message.trim()
    )
    .ok()?;
    Some(path)
}

/// Sends panics to the log too (a panic in a windowless process would otherwise vanish).
pub fn install_panic_hook() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        record(&format!("panic: {info}"));
        previous(info);
    }));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // The tests change CHXCHX_TECH_HOME, which is process-wide.
    static ENV: Mutex<()> = Mutex::new(());

    fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("chxchx-startup-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn a_failure_is_appended_under_the_application_home() {
        let _guard = ENV.lock().unwrap_or_else(|error| error.into_inner());
        let home = scratch("append");
        unsafe { std::env::set_var("CHXCHX_TECH_HOME", &home) };

        let first = record("No se pudo abrir la ventana").expect("the log is written");
        record("segundo intento\n");

        assert_eq!(first, home.join("logs").join("studio-startup.log"));
        let text = fs::read_to_string(&first).unwrap();
        assert_eq!(text.lines().count(), 2);
        assert!(text.contains("No se pudo abrir la ventana") && text.contains("segundo intento"));
        assert!(text.lines().all(|line| line.contains("chxchx-studio ")));
        unsafe { std::env::remove_var("CHXCHX_TECH_HOME") };
    }

    #[test]
    fn a_log_past_the_limit_starts_again() {
        let _guard = ENV.lock().unwrap_or_else(|error| error.into_inner());
        let home = scratch("rotate");
        unsafe { std::env::set_var("CHXCHX_TECH_HOME", &home) };
        let path = log_path().unwrap();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, vec![b'x'; (MAX_BYTES + 1) as usize]).unwrap();

        record("nuevo").unwrap();

        let text = fs::read_to_string(&path).unwrap();
        assert!(text.len() < 200 && text.contains("nuevo"));
        unsafe { std::env::remove_var("CHXCHX_TECH_HOME") };
    }
}
