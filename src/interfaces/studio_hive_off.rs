//! Colmena is not part of this build (`--no-default-features`): nothing to show.
use super::*;

#[derive(Default)]
pub(super) struct State;

#[derive(Clone, Debug)]
pub(super) enum Event {}

pub(super) fn visible(_: &Dashboard) -> bool {
    false
}

pub(super) fn refresh(_: &mut Dashboard) -> Task<Message> {
    Task::none()
}

pub(super) fn update(_: &mut Dashboard, event: Event) -> Task<Message> {
    match event {}
}

pub(super) fn view(_: &Dashboard) -> Element<'_, Message> {
    Space::new().into()
}

pub(super) fn board_view(_: &Dashboard) -> Element<'_, Message> {
    Space::new().into()
}

pub(super) fn takes_center(_: &Dashboard) -> bool {
    false
}
