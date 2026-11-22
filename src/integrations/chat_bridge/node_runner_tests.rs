use super::*;

fn spec() -> TurnSpec {
    TurnSpec {
        agent: "codex".into(),
        provider: "Codex".into(),
        project: PathBuf::from("/p"),
        prompt: "hola".into(),
        mode: "workspace-write".into(),
        agent_command: vec!["codex".into()],
        executable: Some(PathBuf::from("/bin/codex")),
        agent_cwd: PathBuf::from("/p"),
        trusted: true,
        ..Default::default()
    }
}

#[test]
fn a_plain_turn_of_a_default_agent_can_go_to_the_node() {
    assert!(delegable(&spec()));
}

#[test]
fn anything_the_node_would_lose_keeps_the_turn_on_the_local_path() {
    let mut with_images = spec();
    with_images.images = vec![PathBuf::from("/tmp/a.png")];
    let mut with_recap = spec();
    with_recap.resume_recap = "resumen".into();
    let mut with_memories = spec();
    with_memories.recovered_memories = "recuerdos".into();
    let mut with_rules = spec();
    with_rules.allow = vec!["Bash(ls)".into()];
    let mut custom_command = spec();
    custom_command.agent_command = vec!["codex".into(), "--profile".into(), "x".into()];
    let mut other_program = spec();
    other_program.agent_command = vec!["claude".into()];
    let mut elsewhere = spec();
    elsewhere.agent_cwd = PathBuf::from("/otra");
    let mut unknown_provider = spec();
    unknown_provider.provider = "Agente".into();
    let mut odd_mode = spec();
    odd_mode.mode = "root".into();
    for (name, turn) in [
        ("images", with_images),
        ("recap", with_recap),
        ("memories", with_memories),
        ("rules", with_rules),
        ("custom command", custom_command),
        ("another program", other_program),
        ("another folder", elsewhere),
        ("unknown provider", unknown_provider),
        ("odd mode", odd_mode),
    ] {
        assert!(!delegable(&turn), "{name} must stay local");
    }
}

#[test]
fn job_events_become_what_the_chat_already_understands() {
    let event = |body| JobEvent {
        job: JobId::parse("j").unwrap(),
        seq: 1,
        at_ms: 0,
        body,
    };
    let agent = serde_json::json!({"schema": "chxchx.agent-event", "schema_version": 1, "type": "message", "role": "assistant", "text": "x"});
    let (shown, end) = translate(event(JobEventBody::Agent {
        event: agent.clone(),
    }));
    assert_eq!((shown, end), (Some(agent), None));
    let (notice, _) = translate(event(JobEventBody::Progress {
        message: "ojo".into(),
        fraction: None,
    }));
    let notice = notice.unwrap();
    assert_eq!(
        (notice["type"].as_str(), notice["message"].as_str()),
        (Some("notice"), Some("ojo"))
    );
    assert_eq!(notice["schema"], "chxchx.agent-event");
    let error = chxchx_protocol::JobError::new(chxchx_protocol::ErrorCode::ProviderFailed, "falló");
    let (shown, _) = translate(event(JobEventBody::Error { error }));
    assert_eq!(shown.unwrap()["type"], "error");
    for (state, expected) in [
        (JobState::Queued, None),
        (JobState::Running, None),
        (JobState::Succeeded, Some((0, false))),
        (JobState::Cancelled, Some((0, true))),
        (JobState::Failed, Some((1, false))),
        (JobState::Interrupted, Some((1, false))),
    ] {
        assert_eq!(
            translate(event(JobEventBody::State { state })),
            (None, expected),
            "{state:?}"
        );
    }
}
