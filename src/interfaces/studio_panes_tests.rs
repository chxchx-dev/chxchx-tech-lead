use super::*;

fn chat(id: &str) -> Content {
    Content::Chat(id.into())
}

#[test]
fn chats_open_as_tabs_of_the_group_in_use_and_a_second_request_reuses_the_tab() {
    let mut state = State::default();
    let pane = state.show(chat("claude"));
    state.show(chat("codex"));
    assert_eq!(state.grid.len(), 1, "no new window: they are tabs");
    let group = state.grid.get(pane).unwrap();
    assert_eq!(group.tabs, [Content::Editor, chat("claude"), chat("codex")]);
    assert_eq!(group.current(), Some(&chat("codex")));
    state.show(chat("claude"));
    assert_eq!(
        state.grid.get(pane).unwrap().current(),
        Some(&chat("claude"))
    );
    assert_eq!(state.grid.get(pane).unwrap().tabs.len(), 3);
    assert!(
        state.find(|c| *c == chat("claude")).is_some()
            && state.shows_chat("claude")
            && !state.shows_chat("codex")
    );
}

#[test]
fn terminals_gather_in_one_group_under_the_editor() {
    let mut state = State::default();
    state.show(Content::Terminal(None));
    state.attach_terminal(1);
    assert_eq!(state.grid.len(), 2, "the placeholder became the session");
    state.attach_terminal(2);
    assert_eq!(
        (state.grid.len(), state.terminal_panes()),
        (2, 2),
        "the second is a tab of the same group"
    );
    state.show(Content::Activity);
    assert!(state.has_activity());
}

#[test]
fn a_tab_moves_out_to_its_own_pane_and_a_pane_dropped_on_another_becomes_tabs() {
    let mut state = State::default();
    let editor = state.show(chat("claude"));
    state.move_out(editor, Axis::Vertical);
    assert_eq!(state.grid.len(), 2);
    assert_eq!(state.grid.get(editor).unwrap().tabs, [Content::Editor]);
    let chat_pane = state.find(|c| *c == chat("claude")).unwrap();
    state.merge(chat_pane, editor);
    assert_eq!(state.grid.len(), 1);
    assert_eq!(
        state.grid.get(editor).unwrap().tabs,
        [Content::Editor, chat("claude")]
    );
}

#[test]
fn closing_tabs_keeps_the_editor_and_drops_groups_that_end_up_empty() {
    let mut state = State::default();
    state.attach_terminal(1);
    state.show(chat("codex"));
    state.hide(|_| true);
    assert_eq!(state.grid.len(), 1);
    assert_eq!(state.grid.iter().next().unwrap().1.tabs, [Content::Editor]);
    assert_eq!(state.focus, None);

    let mut state = State::default();
    let pane = state.show(chat("a"));
    state.remove_tab(pane, 1);
    assert_eq!(state.grid.get(pane).unwrap().tabs, [Content::Editor]);
    assert_eq!(state.grid.get(pane).unwrap().active, 0);
}
