//! Wakes the interface when something it waits for has data (a terminal printed, an agent
//! answered), so it can sleep the rest of the time.
//!
//! Why: polling on a timer rebuilds and redraws the whole window every tick, even when nothing
//! changed, and on a modest machine that is most of the CPU an idle app uses. With a wake-up the
//! app does nothing until there is work, and the reader threads (which already block on the pipe)
//! are the ones that say so.
use std::sync::OnceLock;

type Hook = Box<dyn Fn() + Send + Sync>;

static HOOK: OnceLock<Hook> = OnceLock::new();

/// The interface registers how it wants to be woken. Only the first call counts.
pub fn set_hook(hook: impl Fn() + Send + Sync + 'static) {
    let _ = HOOK.set(Box::new(hook));
}

/// Called from reader threads when new data arrived. Cheap and safe to call often.
pub fn notify() {
    if let Some(hook) = HOOK.get() {
        hook();
    }
}

/// After an end of stream the process may still be finishing: wake again shortly so the exit is seen.
pub fn notify_after_end() {
    notify();
    std::thread::sleep(std::time::Duration::from_millis(250));
    notify();
}
