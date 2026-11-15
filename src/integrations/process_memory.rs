//! Memory of the Studio process and of the programs it starts: how much it holds, how to hand
//! back what it no longer uses, and the settings that keep child processes lean.
//!
//! Why this exists (the short version; `MEMORIA.md` has the long one): a program that frees
//! memory does not always give it back to the operating system. The allocator keeps freed pages
//! "just in case", so the number Task Manager shows (resident memory) stays high after a busy
//! moment. Giving the pages back is cheap if done at calm moments, so Studio does it on a timer
//! and after closing something big.

/// Settings every program Studio starts gets. Both matter only to glibc (Linux) and are
/// ignored elsewhere:
/// - `MALLOC_ARENA_MAX=2`: glibc gives each thread its own arena; a multi-threaded agent ends up
///   holding many half-empty ones. Two are plenty for what an agent does.
/// - `MALLOC_TRIM_THRESHOLD_`: return freed memory at the top of the heap once 128 KiB are free.
pub const LEAN_ENV: [(&str, &str); 2] = [
    ("MALLOC_ARENA_MAX", "2"),
    ("MALLOC_TRIM_THRESHOLD_", "131072"),
];

/// Limits glibc to two malloc arenas for this very process. Call it first thing in `main`,
/// before any thread exists.
///
/// Each thread that allocates gets an arena, and glibc *reserves* 64 MiB of address space for
/// each one (not RAM: the pages cost nothing until used). With a dozen threads that adds up to
/// ~900 MiB of "virtual size", which is the number some system monitors show as the app's
/// memory. Two arenas keep it near 100 MiB and also reduce fragmentation.
pub fn lean_allocator() {
    imp::lean_allocator();
}

/// Resident memory (what the process really holds in RAM) in bytes, if the system tells.
pub fn resident_bytes() -> Option<u64> {
    imp::resident_bytes()
}

/// Asks the allocator to return freed pages to the system. Safe to call any time; it can
/// take a few milliseconds, so call it from a thread, not from the UI.
pub fn give_back() {
    imp::give_back();
}

/// [`give_back`] on its own thread, for the moments right after something big was freed.
pub fn give_back_soon() {
    std::thread::spawn(give_back);
}

/// "58 MB", "1.2 GB".
pub fn format_bytes(bytes: u64) -> String {
    const MB: f64 = 1024.0 * 1024.0;
    let value = bytes as f64 / MB;
    if value >= 1024.0 {
        format!("{:.1} GB", value / 1024.0)
    } else {
        format!("{value:.0} MB")
    }
}

#[cfg(target_os = "linux")]
mod imp {
    pub fn resident_bytes() -> Option<u64> {
        let status = std::fs::read_to_string("/proc/self/status").ok()?;
        let line = status.lines().find(|line| line.starts_with("VmRSS:"))?;
        let kilobytes: u64 = line.split_whitespace().nth(1)?.parse().ok()?;
        Some(kilobytes * 1024)
    }

    #[cfg(target_env = "gnu")]
    pub fn lean_allocator() {
        unsafe extern "C" {
            fn mallopt(parameter: i32, value: i32) -> i32;
        }
        const M_ARENA_MAX: i32 = -8;
        // SAFETY: `mallopt` only sets an allocator tunable; called before other threads start.
        unsafe {
            mallopt(M_ARENA_MAX, 2);
        }
    }

    #[cfg(not(target_env = "gnu"))]
    pub fn lean_allocator() {}

    #[cfg(target_env = "gnu")]
    pub fn give_back() {
        unsafe extern "C" {
            fn malloc_trim(pad: usize) -> i32;
        }
        // SAFETY: `malloc_trim` is thread-safe and only releases memory the allocator already freed.
        unsafe {
            malloc_trim(0);
        }
    }

    #[cfg(not(target_env = "gnu"))]
    pub fn give_back() {}
}

#[cfg(windows)]
mod imp {
    pub fn lean_allocator() {}

    #[repr(C)]
    #[derive(Default)]
    struct Counters {
        size: u32,
        page_faults: u32,
        peak_working_set: usize,
        working_set: usize,
        quota_peak_paged: usize,
        quota_paged: usize,
        quota_peak_non_paged: usize,
        quota_non_paged: usize,
        pagefile: usize,
        peak_pagefile: usize,
    }

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetCurrentProcess() -> isize;
        fn K32GetProcessMemoryInfo(process: isize, counters: *mut Counters, size: u32) -> i32;
        fn SetProcessWorkingSetSize(process: isize, minimum: usize, maximum: usize) -> i32;
    }

    pub fn resident_bytes() -> Option<u64> {
        let mut counters = Counters {
            size: std::mem::size_of::<Counters>() as u32,
            ..Counters::default()
        };
        // SAFETY: `counters` is a valid, correctly sized PROCESS_MEMORY_COUNTERS for this call.
        let ok =
            unsafe { K32GetProcessMemoryInfo(GetCurrentProcess(), &mut counters, counters.size) };
        (ok != 0).then_some(counters.working_set as u64)
    }

    /// Windows keeps freed pages in the "working set"; asking for (-1, -1) trims it.
    pub fn give_back() {
        // SAFETY: trimming the working set of the current process has no preconditions.
        unsafe {
            SetProcessWorkingSetSize(GetCurrentProcess(), usize::MAX, usize::MAX);
        }
    }
}

#[cfg(not(any(target_os = "linux", windows)))]
mod imp {
    pub fn lean_allocator() {}

    pub fn resident_bytes() -> Option<u64> {
        let output = std::process::Command::new("ps")
            .args(["-o", "rss=", "-p", &std::process::id().to_string()])
            .output()
            .ok()?;
        let kilobytes: u64 = String::from_utf8_lossy(&output.stdout)
            .trim()
            .parse()
            .ok()?;
        Some(kilobytes * 1024)
    }

    pub fn give_back() {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes_are_written_for_people() {
        assert_eq!(format_bytes(58 * 1024 * 1024), "58 MB");
        assert_eq!(format_bytes(1536 * 1024 * 1024), "1.5 GB");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn freed_memory_is_handed_back_to_the_system() {
        let before = resident_bytes().expect("linux reports it");
        let big: Vec<Vec<u8>> = (0..64).map(|i| vec![i as u8; 1024 * 1024]).collect();
        let held = resident_bytes().unwrap();
        assert!(held >= before + 48 * 1024 * 1024, "{before} -> {held}");
        drop(big);
        give_back();
        let after = resident_bytes().unwrap();
        assert!(after + 32 * 1024 * 1024 < held, "{held} -> {after}");
    }
}
