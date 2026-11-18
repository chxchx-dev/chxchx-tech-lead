use super::*;
use serde_json::json;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        let id = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "chxchx-chat-history-{name}-{}-{id}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn write(&self, relative: &str, content: &str) -> PathBuf {
        let path = self.0.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, content).unwrap();
        path
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn jsonl(records: impl IntoIterator<Item = Value>) -> String {
    records
        .into_iter()
        .map(|record| record.to_string())
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn reads_codex_and_claude_histories_only_for_the_selected_project() {
    let project = Scratch::new("project");
    let codex_data = Scratch::new("codex");
    let claude = Scratch::new("claude");
    project.write("src/main.rs", "");
    codex_data.write(
        "sessions/2026/11/rollout-codex-1.jsonl",
        &jsonl([
            json!({"type":"session_meta", "payload":{"id":"codex-session", "cwd":project.0}}),
            json!({"type":"event_msg", "payload":{"type":"user_message", "message":"Fix React Native startup"}}),
            json!({"type":"response_item", "payload":{"type":"message", "role":"assistant", "content":[{"type":"output_text", "text":"Use pnpm start."}]}}),
        ]),
    );
    let claude_key = project
        .0
        .to_string_lossy()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                '-'
            }
        })
        .collect::<String>();
    claude.write(
        &format!("projects/{claude_key}/claude-session.jsonl"),
        &jsonl([
            json!({"type":"user", "cwd":project.0, "message":{"role":"user", "content":"Review the auth flow"}}),
            json!({"type":"assistant", "message":{"role":"assistant", "content":[{"type":"text", "text":"I found two issues."}]}}),
        ]),
    );

    let conversations = list_conversations_in(&project.0, &codex_data.0, &claude.0, "", 20);

    assert_eq!(conversations.len(), 2);
    assert_eq!(
        conversations
            .iter()
            .map(|item| item.provider.as_str())
            .collect::<std::collections::BTreeSet<_>>(),
        ["Claude", "Codex"].into()
    );
    assert_eq!(
        list_conversations_in(&project.0, &codex_data.0, &claude.0, "pnpm", 20).len(),
        1
    );
    let codex_conversation = conversations
        .iter()
        .find(|item| item.provider == "Codex")
        .unwrap();
    assert_eq!(codex_conversation.session_id, "codex-session");
    assert_eq!(codex_conversation.title, "Fix React Native startup");
    assert_eq!(
        codex_conversation.messages.last().unwrap().text,
        "Use pnpm start."
    );
    assert_eq!(codex_conversation.preview(), "Fix React Native startup");
}

#[test]
fn claude_title_sidechains_and_project_path_collisions_are_handled() {
    let project = Scratch::new("my_app.v2");
    let other = Scratch::new("my-app");
    let codex = Scratch::new("codex");
    let claude = Scratch::new("claude");
    let key = |path: &Path| {
        path.to_string_lossy()
            .chars()
            .map(|character| {
                if character.is_ascii_alphanumeric() {
                    character
                } else {
                    '-'
                }
            })
            .collect::<String>()
    };
    let folder = format!("projects/{}", key(&project.0));
    claude.write(
        &format!("{folder}/mine.jsonl"),
        &jsonl([
            json!({"type":"custom-title", "customTitle":"", "title":"Proyecto propio"}),
            json!({"type":"user", "cwd":project.0, "message":{"role":"user", "content":"Mi solicitud"}}),
            json!({"type":"assistant", "isSidechain":true, "message":{"content":"No incluir"}}),
            json!({"type":"assistant", "message":{"role":"assistant", "content":[{"type":"tool_use", "name":"Read"},{"type":"text", "text":"Listo."}]}}),
        ]),
    );
    claude.write(
        &format!("{folder}/other.jsonl"),
        &jsonl([json!({"type":"user", "cwd":other.0, "message":{"content":"Otro proyecto"}})]),
    );

    let conversations = list_conversations_in(&project.0, &codex.0, &claude.0, "", 20);

    assert_eq!(conversations.len(), 1);
    assert_eq!(conversations[0].title, "Proyecto propio");
    assert_eq!(
        conversations[0]
            .messages
            .iter()
            .map(|message| message.text.as_str())
            .collect::<Vec<_>>(),
        ["Mi solicitud", "[Herramienta: Read]\nListo."]
    );
}

#[test]
fn malformed_records_empty_histories_and_limits_are_safe() {
    let project = Scratch::new("project");
    let codex = Scratch::new("codex");
    let claude = Scratch::new("claude");
    let path = codex.write(
        "sessions/rollout-large.jsonl",
        &format!(
            "not json\n{}\n{}\n{}",
            json!({"type":"session_meta", "payload":{"cwd":project.0}}),
            json!({"type":"event_msg", "payload":{"type":"user_message", "message":"{}", "ignored":true}}),
            json!({"type":"event_msg", "payload":{"type":"agent_message", "message":"done"}}),
        ),
    );
    let conversations = list_conversations_in(&project.0, &codex.0, &claude.0, "", 1);

    assert_eq!(conversations.len(), 1);
    assert_eq!(conversations[0].messages.len(), 2);
    assert!(conversations[0].messages[0].text.len() <= MAX_MESSAGE_CHARS);
    assert!(list_conversations_in(&project.0, &codex.0, &claude.0, "missing", 10).is_empty());
    assert!(
        list_conversations_in(&project.0.join("missing"), &codex.0, &claude.0, "", 10).is_empty()
    );
    assert!(path.exists());
}

#[cfg(unix)]
#[test]
fn transcript_discovery_does_not_follow_symlinks() {
    use std::os::unix::fs::symlink;

    let project = Scratch::new("project");
    let codex = Scratch::new("codex");
    let claude = Scratch::new("claude");
    let outside = Scratch::new("outside");
    let external = outside.write(
        "rollout-external.jsonl",
        &jsonl([
            json!({"type":"session_meta", "payload":{"id":"external", "cwd":project.0}}),
            json!({"type":"event_msg", "payload":{"type":"user_message", "message":"secreto fuera"}}),
        ]),
    );
    fs::create_dir_all(codex.0.join("sessions")).unwrap();
    symlink(&external, codex.0.join("sessions/rollout-linked.jsonl")).unwrap();

    let conversations = list_conversations_in(&project.0, &codex.0, &claude.0, "", 20);

    assert!(conversations.is_empty());
}
