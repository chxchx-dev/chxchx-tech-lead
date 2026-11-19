use super::*;
use std::fs;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        let id = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "chxchx-opencode-history-{name}-{}-{id}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn list(root: &Path, id: &str, updated: Value) -> Value {
    serde_json::json!([{
        "id": id,
        "directory": root,
        "updated": updated.clone(),
        "time": {"updated": updated},
    }])
}

#[test]
fn reads_version_two_exports_only_from_the_selected_project() {
    let project = Scratch::new("v2-project");
    let other = Scratch::new("v2-other");
    let executable = PathBuf::from("/usr/bin/opencode");
    let exported = serde_json::json!({
        "info": {"id":"ses_two", "location":{"directory":project.0}},
        "messages": [
            {"type":"agent-switched", "agent":"build"},
            {"type":"user", "text":"Recuerda que staging usa Postgres 16."},
            {"type":"assistant", "content":[{"type":"tool", "name":"shell"}, {"type":"text", "text":"Listo."}]},
            {"type":"idle"}
        ]
    });
    let mut calls = Vec::new();
    let conversations =
        recent_user_conversations_with(&project.0, &executable, 10, |command, cwd| {
            calls.push(command.to_vec());
            assert_eq!(cwd, project.0);
            match command.get(1..3) {
                Some([first, second]) if first == "session" && second == "list" => {
                    assert!(
                        command
                            .windows(2)
                            .any(|pair| pair[0] == "--max-count" && pair[1] == "10")
                    );
                    Some(serde_json::json!([
                        {"id":"foreign", "directory":other.0, "updated":1},
                        {"id":"ses_two", "directory":project.0, "updated":1_800_000_000_000_i64}
                    ]))
                }
                Some([first, second]) if first == "session" && second == "export" => {
                    Some(exported.clone())
                }
                _ => None,
            }
        });

    assert_eq!(calls.len(), 2);
    assert_eq!(
        calls[1][1..4]
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["session", "export", "ses_two"]
    );
    assert_eq!(conversations.len(), 1);
    assert_eq!(conversations[0].session_id, "ses_two");
    assert_eq!(conversations[0].provider, "OpenCode");
    assert_eq!(conversations[0].messages.len(), 1);
    assert_eq!(
        conversations[0].messages[0].text,
        "Recuerda que staging usa Postgres 16."
    );
    assert_eq!(
        conversations[0]
            .modified_at
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        1_800_000_000
    );
}

#[test]
fn falls_back_to_the_one_point_x_export_and_reads_only_user_text_parts() {
    let project = Scratch::new("v1-project");
    let executable = PathBuf::from("/usr/bin/opencode");
    let exported = serde_json::json!({
        "info":{"directory":project.0},
        "messages":[
            {"info":{"role":"user"}, "parts":[{"type":"text", "text":"Recuerda "}, {"type":"tool"}, {"type":"text", "text":"que usamos pnpm."}]},
            {"info":{"role":"assistant"}, "parts":[{"type":"text", "text":"Anotado."}]}
        ]
    });
    let mut calls = Vec::new();
    let conversations =
        recent_user_conversations_with(&project.0, &executable, 10, |command, _| {
            calls.push(command.to_vec());
            match command.get(1..3) {
                Some([first, second]) if first == "session" && second == "list" => Some(list(
                    &project.0,
                    "ses_one",
                    serde_json::json!(1_800_000_000),
                )),
                Some([first, second]) if first == "session" && second == "export" => None,
                Some([first, _]) if first == "export" => Some(exported.clone()),
                _ => None,
            }
        });

    assert_eq!(calls.len(), 3);
    assert_eq!(
        calls[1][1..3]
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["session", "export"]
    );
    assert_eq!(
        calls[2][1..3]
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["export", "ses_one"]
    );
    assert_eq!(conversations.len(), 1);
    assert_eq!(conversations[0].messages.len(), 1);
    assert_eq!(
        conversations[0].messages[0].text,
        "Recuerda \nque usamos pnpm."
    );
    assert_eq!(
        conversations[0]
            .modified_at
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        1_800_000_000
    );
}

#[test]
fn rejects_invalid_ids_mismatched_export_directories_and_unbounded_limits() {
    let project = Scratch::new("reject-project");
    let other = Scratch::new("reject-other");
    let executable = PathBuf::from("/usr/bin/opencode");
    let mut exported_count = 0;
    let conversations =
        recent_user_conversations_with(&project.0, &executable, usize::MAX, |command, _| {
            match command.get(1..3) {
                Some([first, second]) if first == "session" && second == "list" => {
                    Some(serde_json::json!([
                        {"id":"bad/id", "directory":project.0},
                        {"id":"ses_wrong", "directory":other.0},
                        {"id":"ses_export_wrong", "directory":project.0}
                    ]))
                }
                Some([first, second]) if first == "session" && second == "export" => {
                    exported_count += 1;
                    Some(serde_json::json!({
                        "info":{"location":{"directory":other.0}},
                        "messages":[{"type":"user", "text":"Recuerda algo."}]
                    }))
                }
                _ => None,
            }
        });

    assert_eq!(exported_count, 1);
    assert!(conversations.is_empty());
}

#[cfg(unix)]
#[test]
fn opencode_history_runs_through_the_bounded_cli_adapter() {
    use std::os::unix::fs::PermissionsExt;

    let project = Scratch::new("cli-project");
    let executable = project.0.join("fake-opencode");
    let list_path = project.0.join("session-list.json");
    let export_path = project.0.join("session-export.json");
    fs::write(
        &list_path,
        serde_json::json!([{
            "id":"cli-session",
            "directory":project.0,
            "updated":1_800_000_000_i64
        }])
        .to_string(),
    )
    .unwrap();
    fs::write(
        &export_path,
        serde_json::json!({
            "info":{"directory":project.0},
            "messages":[{"info":{"role":"user"}, "parts":[{"type":"text", "text":"Recuerda el puerto 8080."}]}]
        })
        .to_string(),
    )
    .unwrap();
    let script = format!(
        "#!/bin/sh\nif [ \"$1 $2\" = \"session list\" ]; then cat '{}'\nelif [ \"$1 $2\" = \"session export\" ]; then cat '{}'\nelse exit 2\nfi\n",
        list_path.display(),
        export_path.display()
    );
    fs::write(&executable, script).unwrap();
    let mut permissions = fs::metadata(&executable).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&executable, permissions).unwrap();

    let conversations = recent_user_conversations_with(&project.0, &executable, 10, run_json);

    assert_eq!(conversations.len(), 1);
    assert_eq!(
        conversations[0].messages[0].text,
        "Recuerda el puerto 8080."
    );
}
