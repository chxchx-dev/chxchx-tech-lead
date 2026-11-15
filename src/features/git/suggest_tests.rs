use super::rules::{Class, classify};
use super::*;

fn change(path: &str, badge: char, additions: u32, deletions: u32) -> ChangeInfo {
    ChangeInfo {
        path: path.to_owned(),
        orig_path: None,
        badge,
        additions,
        deletions,
        fix_hints: 0,
        sensitive: is_sensitive(path),
    }
}

#[test]
fn files_are_classified_by_what_they_are() {
    let class = |path| classify(path);
    assert_eq!(class("docs/guia.md"), Class::Docs);
    assert_eq!(class(".ai/HANDOFF.md"), Class::Docs);
    assert_eq!(class("README.md"), Class::Docs);
    assert_eq!(class("tests/test_api.py"), Class::Test);
    assert_eq!(class("src/app/widget.test.ts"), Class::Test);
    assert_eq!(class(".github/workflows/ci.yml"), Class::Ci);
    assert_eq!(class("migration/rust/Cargo.lock"), Class::Build);
    assert_eq!(class("requirements-dev.txt"), Class::Build);
    assert_eq!(class(".gitignore"), Class::Chore);
    assert_eq!(class("assets/logo.png"), Class::Chore);
    assert_eq!(class("src/main.rs"), Class::Source);
}

#[test]
fn a_mixed_set_is_split_into_conventional_commits() {
    let changes = vec![
        change("migration/rust/src/git.rs", 'N', 120, 0),
        change("migration/rust/src/git_view.rs", 'N', 80, 0),
        change("migration/rust/src/lib.rs", 'M', 3, 0),
        change("migration/rust/tests/git.rs", 'N', 60, 0),
        change("src/chxchx_tech_lead/core/mods.py", 'M', 4, 4),
        change(".ai/CURRENT_STATE.md", 'M', 20, 1),
        change("README.md", 'M', 5, 0),
        change("migration/rust/Cargo.lock", 'M', 30, 10),
        change(".github/workflows/ci.yml", 'M', 2, 1),
        change(".gitignore", 'M', 1, 0),
    ];
    let plan = plan(&changes);
    let subjects: Vec<String> = plan.groups.iter().map(CommitGroup::subject).collect();
    assert!(
        subjects.iter().any(|s| s.starts_with("build")),
        "{subjects:?}"
    );
    assert!(
        subjects.iter().any(|s| s.starts_with("feat(rust): añade")),
        "{subjects:?}"
    );
    assert!(
        subjects.iter().any(|s| s.starts_with("test(rust)")),
        "{subjects:?}"
    );
    assert!(
        subjects
            .iter()
            .any(|s| s.starts_with("fix(chxchx_tech_lead)")),
        "small edit: {subjects:?}"
    );
    assert!(
        subjects.iter().any(|s| s.starts_with("docs")),
        "{subjects:?}"
    );
    assert!(subjects.iter().any(|s| s.starts_with("ci")), "{subjects:?}");
    assert!(
        subjects.iter().any(|s| s.starts_with("chore")),
        "{subjects:?}"
    );
    let every_file: usize = plan.groups.iter().map(|group| group.files.len()).sum();
    assert_eq!(
        every_file,
        changes.len(),
        "each file lands in exactly one commit"
    );
    assert!(
        plan.groups
            .iter()
            .all(|group| group.subject().chars().count() <= MAX_SUBJECT)
    );
    assert_eq!(
        plan.groups[0].kind,
        CommitType::Build,
        "dependencies go first"
    );
}

#[test]
fn fix_words_and_deletions_change_the_type() {
    let mut fixed = change("src/lib.rs", 'M', 40, 30);
    fixed.fix_hints = 1;
    assert_eq!(plan(&[fixed]).groups[0].kind, CommitType::Fix);
    let removed = change("src/old.rs", 'D', 0, 90);
    let group = &plan(&[removed]).groups[0];
    assert_eq!(group.kind, CommitType::Refactor);
    assert!(group.text.starts_with("elimina"));
    assert_eq!(fix_words("prefix fixture"), 0, "whole words only");
    assert_eq!(fix_words("// fix: corrige el bug"), 3);
}

#[test]
fn secrets_are_flagged_and_single_suggestions_report_mixed_topics() {
    let changes = vec![change(".env", 'N', 3, 0), change("src/a.rs", 'M', 10, 2)];
    let plan = plan(&changes);
    assert!(plan.warnings.iter().any(|warning| warning.contains(".env")));
    for path in [
        "id_rsa",
        "config/server.pem",
        ".aws/credentials",
        "secrets.yml",
    ] {
        assert!(is_sensitive(path), "{path}");
    }
    assert!(
        !is_sensitive("src/secretary.rs"),
        "only secret(s) files, not any name that starts that way"
    );

    let one = suggest(&[change("src/a.rs", 'M', 5, 5)]).unwrap();
    assert!(!one.1);
    let many = suggest(&[
        change("src/a.rs", 'M', 5, 5),
        change("README.md", 'M', 2, 0),
        change("tests/t.rs", 'N', 9, 0),
    ])
    .unwrap();
    assert!(many.1, "several topics are reported as mixed");
    assert!(suggest(&[]).is_none());
}

#[test]
fn a_big_module_is_split_by_sub_module_and_name_family() {
    let mut changes = Vec::new();
    for name in [
        "types",
        "parse",
        "graph",
        "guide",
        "plan",
        "suggest",
        "problem",
        "repository",
    ] {
        changes.push(change(
            &format!("demo/src/features/git/{name}.rs"),
            'N',
            40,
            0,
        ));
    }
    for family in [
        "studio_git",
        "studio_menu",
        "studio_catalog",
        "studio_search",
    ] {
        for suffix in ["", "_ops", "_cmds", "_view"] {
            changes.push(change(
                &format!("demo/src/interfaces/{family}{suffix}.rs"),
                'N',
                30,
                0,
            ));
        }
    }
    changes.push(change("demo/src/lib.rs", 'M', 2, 0));
    let plan = plan(&changes);
    let subjects: Vec<String> = plan.groups.iter().map(CommitGroup::subject).collect();
    assert!(
        subjects.iter().any(|s| s.starts_with("feat(git)")),
        "{subjects:?}"
    );
    assert!(
        subjects
            .iter()
            .any(|s| s.starts_with("feat(interfaces): añade studio_git y archivos relacionados")),
        "{subjects:?}"
    );
    assert!(
        plan.groups.iter().all(|group| group.files.len() <= 12),
        "{subjects:?}"
    );
    assert!(plan.warnings.is_empty(), "{:?}", plan.warnings);
    let total: usize = plan.groups.iter().map(|group| group.files.len()).sum();
    assert_eq!(total, changes.len());
}

#[test]
fn conventional_subjects_are_recognised() {
    for good in [
        "feat: añade git",
        "fix(rust): corrige",
        "refactor(a)!: x",
        "revert: y",
    ] {
        assert!(is_conventional(good), "{good}");
    }
    for bad in [
        "Añade git",
        "feature: x",
        "fix(rust: x",
        "fix:",
        "fix: ",
        "feat(a)(b): x",
    ] {
        assert!(!is_conventional(bad), "{bad}");
    }
}

#[test]
fn the_type_can_be_cycled_and_the_message_has_subject_and_body() {
    let mut group = plan(&[change("src/a.rs", 'N', 5, 0)]).groups.remove(0);
    assert_eq!(group.kind, CommitType::Feat);
    group.kind = group.kind.next();
    assert_eq!(group.kind, CommitType::Fix);
    assert_eq!(
        CommitType::Chore.next(),
        CommitType::Feat,
        "the cycle wraps around"
    );
    group.text = "corrige el guardado".to_owned();
    let message = group.message();
    assert!(
        message.starts_with("fix: corrige el guardado\n\n- src/a.rs"),
        "{message}"
    );
}
