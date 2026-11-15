fn main() {
    restore_default_sigpipe();
    if let Err(error) = chxchx_tech::interfaces::cli::run() {
        chxchx_tech::interfaces::cli::report_error(&error);
        std::process::exit(1);
    }
}

/// Rust ignores SIGPIPE, so `chxchx-tech skills list | head` would panic on the closed pipe.
/// A command-line tool should simply stop, like every other one.
#[cfg(unix)]
fn restore_default_sigpipe() {
    unsafe extern "C" {
        fn signal(signum: i32, handler: usize) -> usize;
    }
    const SIGPIPE: i32 = 13;
    const SIG_DFL: usize = 0;
    // SAFETY: installing the default disposition for SIGPIPE at startup, before any thread exists.
    unsafe {
        signal(SIGPIPE, SIG_DFL);
    }
}

#[cfg(not(unix))]
fn restore_default_sigpipe() {}
