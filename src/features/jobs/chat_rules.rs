//! What a person allowed "for this chat" from another device (ADR 0017).
//!
//! In Studio the window remembers the rules of the permissions approved with «en este chat» and
//! hands them to the agent on the next turn. A phone has no such memory, so the node keeps it: the
//! rules are the ones the agent itself offered on the card (the client only says "the chat choice"),
//! and they apply to later turns of the *same* conversation: same project, same agent, same session.
//! They live as long as the node does; a new conversation, or a restart of the node, starts clean.

use std::collections::{BTreeSet, HashMap};
use std::sync::{Mutex, OnceLock};

/// Rules kept for one conversation, and conversations kept: bounds, so memory cannot grow forever.
const MAX_RULES: usize = 50;
const MAX_CHATS: usize = 200;

type Key = (String, String, String);

fn store() -> &'static Mutex<HashMap<Key, BTreeSet<String>>> {
    static STORE: OnceLock<Mutex<HashMap<Key, BTreeSet<String>>>> = OnceLock::new();
    STORE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn key(project: &str, provider: &str, session: &str) -> Key {
    (project.to_owned(), provider.to_owned(), session.to_owned())
}

/// The rules allowed so far for this conversation. A turn that starts a conversation (no session
/// yet) has none.
pub fn rules_for(project: &str, provider: &str, session: Option<&str>) -> Vec<String> {
    let Some(session) = session.filter(|session| !session.is_empty()) else {
        return Vec::new();
    };
    store()
        .lock()
        .ok()
        .and_then(|map| map.get(&key(project, provider, session)).cloned())
        .map(|rules| rules.into_iter().collect())
        .unwrap_or_default()
}

/// Remembers `rules` for this conversation. Without a session there is no conversation to attach
/// them to, so nothing is kept.
pub fn remember(project: &str, provider: &str, session: Option<&str>, rules: &[String]) {
    let Some(session) = session.filter(|session| !session.is_empty()) else {
        return;
    };
    let Ok(mut map) = store().lock() else {
        return;
    };
    if map.len() >= MAX_CHATS && !map.contains_key(&key(project, provider, session)) {
        // The oldest is not tracked: dropping an arbitrary conversation only costs one more question.
        if let Some(any) = map.keys().next().cloned() {
            map.remove(&any);
        }
    }
    let kept = map.entry(key(project, provider, session)).or_default();
    for rule in rules.iter().filter(|rule| !rule.trim().is_empty()) {
        if kept.len() < MAX_RULES {
            kept.insert(rule.clone());
        }
    }
}

/// The rules of the choices a card offered to remember for the chat. Only these can be remembered:
/// what an agent did not offer is never allowed on its behalf.
pub fn offered_for_chat(permission: &serde_json::Value) -> Vec<String> {
    permission["choices"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|choice| choice["persist"].as_bool() == Some(true))
        .flat_map(|choice| choice["allow"].as_array().into_iter().flatten())
        .filter_map(|rule| rule.as_str())
        .map(str::to_owned)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn rules_belong_to_one_conversation_and_to_nobody_else() {
        remember("p-uno", "Claude", Some("s1"), &["Bash(touch:*)".into()]);
        assert_eq!(rules_for("p-uno", "Claude", Some("s1")), ["Bash(touch:*)"]);
        assert!(
            rules_for("p-uno", "Claude", Some("s2")).is_empty(),
            "another session"
        );
        assert!(
            rules_for("p-uno", "Codex", Some("s1")).is_empty(),
            "another agent"
        );
        assert!(
            rules_for("p-dos", "Claude", Some("s1")).is_empty(),
            "another project"
        );
        assert!(
            rules_for("p-uno", "Claude", None).is_empty(),
            "a new conversation starts clean"
        );
    }

    #[test]
    fn without_a_session_nothing_is_kept_and_repeats_are_one_rule() {
        remember("p-tres", "Claude", None, &["Bash(ls:*)".into()]);
        remember("p-tres", "Claude", Some(""), &["Bash(ls:*)".into()]);
        assert!(rules_for("p-tres", "Claude", Some("")).is_empty());
        remember(
            "p-tres",
            "Claude",
            Some("s"),
            &["Bash(ls:*)".into(), "Bash(ls:*)".into(), " ".into()],
        );
        remember("p-tres", "Claude", Some("s"), &["Bash(ls:*)".into()]);
        assert_eq!(rules_for("p-tres", "Claude", Some("s")), ["Bash(ls:*)"]);
    }

    #[test]
    fn a_conversation_cannot_collect_unlimited_rules() {
        let many: Vec<String> = (0..MAX_RULES + 20)
            .map(|n| format!("Bash(c{n}:*)"))
            .collect();
        remember("p-cuatro", "Claude", Some("s"), &many);
        assert_eq!(rules_for("p-cuatro", "Claude", Some("s")).len(), MAX_RULES);
    }

    #[test]
    fn only_what_the_card_offered_to_remember_can_be_remembered() {
        let card = json!({"type": "permission", "choices": [
            {"label": "Permitir este comando", "allow": ["Bash(touch /tmp/x)"], "persist": false},
            {"label": "Permitir «touch» en este chat", "allow": ["Bash(touch:*)"], "persist": true},
            {"label": "sin reglas", "allow": [], "persist": true},
        ]});
        assert_eq!(offered_for_chat(&card), ["Bash(touch:*)"]);
        assert!(offered_for_chat(&json!({"type": "permission"})).is_empty());
    }
}
