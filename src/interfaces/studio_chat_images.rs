//! Images of a chat message: picked, dropped or pasted, prepared off the UI thread.
use super::*;
use crate::integrations::chat_images::{self as pictures, Prepared};
use iced::widget::image::Handle;

pub(super) const MAX_IMAGES: usize = 5;

#[derive(Clone)]
pub(in crate::interfaces::studio) struct Image {
    pub prepared: Prepared,
    pub thumb: Handle,
}

impl std::fmt::Debug for Image {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "Image({})", self.prepared.name)
    }
}

impl Image {
    fn new(prepared: Prepared) -> Self {
        let thumb = Handle::from_rgba(
            prepared.thumb_width,
            prepared.thumb_height,
            prepared.thumb_rgba.clone(),
        );
        Self { prepared, thumb }
    }
}

type Loaded = Result<Vec<Prepared>, String>;

fn deliver(id: String, work: impl FnOnce() -> Loaded + Send + 'static) -> Task<Message> {
    Task::perform(
        async move {
            jobs::offload(work)
                .await
                .unwrap_or_else(|| Err("No pude preparar la imagen".into()))
        },
        move |result| Message::Chat(Event::Loaded(id.clone(), result)),
    )
}

fn active_id(state: &Dashboard) -> Option<String> {
    state.chat.active_chat().map(|chat| chat.agent.id.clone())
}

fn load(paths: Vec<PathBuf>) -> Loaded {
    let mut done = Vec::new();
    let mut failures = Vec::new();
    for path in paths {
        match pictures::prepare(&path) {
            Ok(prepared) => done.push(prepared),
            Err(error) => failures.push(error),
        }
    }
    // Decoding a photo takes far more memory than the prepared image keeps: hand it back.
    crate::integrations::process_memory::give_back();
    if done.is_empty() && !failures.is_empty() {
        return Err(failures.remove(0));
    }
    Ok(done)
}

/// The "Adjuntar" button: the system's file picker.
pub(super) fn attach(state: &mut Dashboard) -> Task<Message> {
    let Some(id) = active_id(state) else {
        return Task::none();
    };
    Task::perform(
        async {
            rfd::AsyncFileDialog::new()
                .set_title("Adjuntar imágenes")
                .add_filter("Imágenes", &["png", "jpg", "jpeg", "gif", "webp"])
                .pick_files()
                .await
                .map(|files| files.into_iter().map(|f| f.path().to_path_buf()).collect())
                .unwrap_or_else(Vec::<PathBuf>::new)
        },
        move |paths| Message::Chat(Event::Picked(id.clone(), paths)),
    )
}

pub(super) fn picked(id: String, paths: Vec<PathBuf>) -> Task<Message> {
    if paths.is_empty() {
        return Task::none();
    }
    deliver(id, move || load(paths))
}

pub(super) fn dropped(state: &mut Dashboard, path: PathBuf) -> Task<Message> {
    match (active_id(state), pictures::is_image(&path)) {
        (Some(id), true) => deliver(id, move || load(vec![path])),
        (Some(_), false) => {
            state.message = "Solo se pueden adjuntar imágenes (PNG, JPG, GIF o WebP).".into();
            Task::none()
        }
        _ => Task::none(),
    }
}

/// Ctrl+V: an image on the clipboard is attached; otherwise the text is pasted as usual.
pub(super) fn paste(state: &mut Dashboard) -> Task<Message> {
    let Some(id) = active_id(state) else {
        return Task::none();
    };
    Task::perform(
        async {
            jobs::offload(|| {
                let result = pictures::from_clipboard();
                crate::integrations::process_memory::give_back();
                result
            })
            .await
            .unwrap_or_else(|| Err("No pude leer el portapapeles".into()))
        },
        move |result| match result {
            Ok(Some(prepared)) => Message::Chat(Event::Loaded(id.clone(), Ok(vec![prepared]))),
            Ok(None) => Message::Chat(Event::PasteText),
            Err(error) => Message::Chat(Event::Loaded(id.clone(), Err(error))),
        },
    )
}

pub(super) fn paste_text() -> Task<Message> {
    iced::clipboard::read().map(|text| Message::Chat(Event::Pasted(text)))
}

pub(super) fn loaded(state: &mut Dashboard, id: &str, result: Loaded) {
    match result {
        Ok(images) => {
            let Some(chat) = state.chat.chats.iter_mut().find(|c| c.agent.id == id) else {
                images.iter().for_each(pictures::discard);
                return;
            };
            for prepared in images {
                if chat.images.len() >= MAX_IMAGES {
                    pictures::discard(&prepared);
                    state.message = format!("Máximo {MAX_IMAGES} imágenes por mensaje.");
                } else {
                    chat.images.push(Image::new(prepared));
                }
            }
        }
        Err(error) => state.message = error,
    }
}

pub(super) fn remove(chat: &mut Chat, index: usize) {
    if index < chat.images.len() {
        pictures::discard(&chat.images.remove(index).prepared);
    }
}

/// Short note for the composer: what each image costs the model.
pub(super) fn note(image: &Image) -> String {
    let p = &image.prepared;
    let size = if p.saved() {
        format!("{} → {} KB", p.original_bytes / 1024, p.bytes / 1024)
    } else {
        format!("{} KB", p.bytes / 1024)
    };
    format!("{}×{} · {size} · ~{} tokens", p.width, p.height, p.tokens())
}

/// Sent images no longer need their prepared copy on disk (the thumbnails stay in the chat).
pub(super) fn cleanup(chat: &Chat) {
    for (_, sent) in &chat.sent {
        sent.iter()
            .for_each(|image| pictures::discard(&image.prepared));
    }
}
