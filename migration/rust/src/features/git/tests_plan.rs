//! A planned commit against a real repository.
use super::tests::{repository, write};
use super::*;
use std::fs;

#[test]
fn a_planned_commit_takes_only_its_own_files_and_analysis_counts_lines() {
    use super::suggest::{CommitType, plan};
    let Some((_keep, repo)) = repository(true) else {
        return;
    };
    for (name, text) in [
        ("a.rs", "uno\n"),
        ("b.rs", "uno\n"),
        ("docs/x.md", "# x\n"),
        ("c.rs", "viejo\n"),
    ] {
        write(&repo, name, text);
    }
    repo.stage(&[
        "a.rs".into(),
        "b.rs".into(),
        "docs/x.md".into(),
        "c.rs".into(),
    ])
    .unwrap();
    repo.commit("base", false).unwrap();

    write(&repo, "a.rs", "uno\ndos\ntres\n");
    write(&repo, "b.rs", "otro\n");
    write(&repo, "docs/x.md", "# x\nmás\n");
    write(&repo, "nuevo.rs", "fn a() {}\nfn b() {}\n");
    fs::remove_file(repo.root.join("c.rs")).unwrap();
    repo.stage(&["b.rs".into()]).unwrap(); // already prepared, but not part of the commits below

    let status = repo.status().unwrap();
    let entries: Vec<GitEntry> = status.entries.clone();
    let infos = repo.analyse(&entries, false).unwrap();
    let info = |path: &str| infos.iter().find(|change| change.path == path).unwrap();
    assert_eq!((info("a.rs").additions, info("a.rs").deletions), (2, 0));
    assert_eq!(
        info("nuevo.rs").additions,
        2,
        "untracked files are counted from disk"
    );
    assert_eq!(info("c.rs").badge, 'D');

    let groups = plan(&infos).groups;
    assert!(groups.iter().any(|group| group.kind == CommitType::Docs));
    assert_eq!(
        groups.iter().map(|group| group.files.len()).sum::<usize>(),
        infos.len()
    );

    // The docs commit must not drag along the prepared b.rs or anything else.
    repo.commit_paths("docs: amplía x", &["docs/x.md".to_owned()], false)
        .unwrap();
    let log = repo.log(5).unwrap();
    assert_eq!(log[0].subject, "docs: amplía x");
    let files: Vec<String> = repo
        .commit_files(&log[0].hash)
        .unwrap()
        .into_iter()
        .map(|f| f.path)
        .collect();
    assert_eq!(files, ["docs/x.md"]);
    let status = repo.status().unwrap();
    assert!(
        status.staged_entries().iter().any(|e| e.path == "b.rs"),
        "b.rs stays prepared"
    );
    assert!(
        status
            .changed_entries()
            .iter()
            .any(|e| e.path == "nuevo.rs")
    );

    // A deletion and an untracked file can be committed by name as well.
    repo.commit_paths("refactor: elimina c", &["c.rs".to_owned()], false)
        .unwrap();
    repo.commit_paths("feat: añade nuevo", &["nuevo.rs".to_owned()], false)
        .unwrap();
    let log = repo.log(5).unwrap();
    assert_eq!(repo.commit_files(&log[1].hash).unwrap()[0].status, "D");
    assert_eq!(repo.commit_files(&log[0].hash).unwrap()[0].status, "A");
    assert!(
        repo.commit_paths("  ", &["a.rs".to_owned()], false)
            .is_err()
    );
    assert!(
        repo.commit_paths("x", &["../fuera".to_owned()], false)
            .is_err()
    );
}
