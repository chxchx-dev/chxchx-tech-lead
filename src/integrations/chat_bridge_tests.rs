use super::*;

#[test]
fn saved_chats_are_filtered_from_the_versioned_contract() {
    let saved = parse_saved_chat(r#"{"schema":"chxchx.chat-state","schema_version":1,"session_id":"s","messages":[{"role":"user","text":"a"},{"role":"tool","text":"x"},{"role":"assistant","text":""},{"role":"assistant","text":"b"}]}"#).unwrap();
    assert_eq!(
        saved,
        SavedChat {
            session_id: "s".into(),
            resume_recap: String::new(),
            messages: vec![
                ("user".into(), "a".into()),
                ("assistant".into(), "b".into())
            ]
        }
    );
    assert!(parse_saved_chat(r#"{"schema":"otro"}"#).is_err());
}
