//! Background jobs: heavy file-system work never runs on the UI thread.
use super::*;
use std::sync::{Arc, Mutex};

type Callback = Box<dyn FnOnce(&mut Dashboard) -> Task<Message> + Send>;

/// A finished job, waiting to be applied to the state on the UI thread.
#[derive(Clone)]
pub(super) struct Done(Arc<Mutex<Option<Callback>>>);

impl std::fmt::Debug for Done {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("Done(..)")
    }
}

/// Runs `job` on its own thread and applies `then` to the state when it finishes.
/// A panic in the job is reported instead of freezing or killing the window.
pub(super) fn run<T: Send + 'static>(
    state: &mut Dashboard,
    job: impl FnOnce() -> T + Send + 'static,
    then: impl FnOnce(&mut Dashboard, T) -> Task<Message> + Send + 'static,
) -> Task<Message> {
    state.pending += 1;
    spawn(job, true, then)
}

/// Like [`run`] for housekeeping (saving preferences) that should not show as work.
pub(super) fn quiet<T: Send + 'static>(
    job: impl FnOnce() -> T + Send + 'static,
    then: impl FnOnce(&mut Dashboard, T) -> Task<Message> + Send + 'static,
) -> Task<Message> {
    spawn(job, false, then)
}

fn spawn<T: Send + 'static>(
    job: impl FnOnce() -> T + Send + 'static,
    counted: bool,
    then: impl FnOnce(&mut Dashboard, T) -> Task<Message> + Send + 'static,
) -> Task<Message> {
    let (sender, receiver) = iced::futures::channel::oneshot::channel();
    std::thread::spawn(move || {
        let _ = sender.send(job());
    });
    Task::perform(
        async move {
            let value = receiver.await.ok();
            let callback: Callback = Box::new(move |state: &mut Dashboard| {
                if counted {
                    state.pending = state.pending.saturating_sub(1);
                }
                match value {
                    Some(value) => then(state, value),
                    None => {
                        state.message = "La tarea en segundo plano terminó sin resultado.".into();
                        Task::none()
                    }
                }
            });
            Done(Arc::new(Mutex::new(Some(callback))))
        },
        Message::Done,
    )
}

/// Runs blocking work on its own thread and awaits it without blocking an executor thread.
/// `None` means the work panicked.
pub(super) async fn offload<T: Send + 'static>(
    job: impl FnOnce() -> T + Send + 'static,
) -> Option<T> {
    let (sender, receiver) = iced::futures::channel::oneshot::channel();
    std::thread::spawn(move || {
        let _ = sender.send(job());
    });
    receiver.await.ok()
}

/// Applies a finished job. Each job is applied exactly once.
pub(super) fn finish(state: &mut Dashboard, done: &Done) -> Task<Message> {
    let callback = done.0.lock().ok().and_then(|mut slot| slot.take());
    match callback {
        Some(callback) => callback(state),
        None => Task::none(),
    }
}
