//! Small checks on text and paths used by the suggestions.
use super::CommitType;
use super::rules::file_name;

/// True when the first line looks like `type(scope)!: description`.
pub fn is_conventional(subject: &str) -> bool {
    let Some((head, description)) = subject.split_once(": ") else {
        return false;
    };
    let head = head.strip_suffix('!').unwrap_or(head);
    let kind = head.split('(').next().unwrap_or(head);
    let scope_ok = !head.contains('(') || (head.ends_with(')') && head.matches('(').count() == 1);
    let known = CommitType::ALL
        .iter()
        .any(|candidate| candidate.label() == kind)
        || kind == "revert";
    known && scope_ok && !description.trim().is_empty()
}

/// Paths that must not be read or sent anywhere: keys, tokens and environment files.
pub fn is_sensitive(path: &str) -> bool {
    let lower = path.to_lowercase();
    let name = file_name(&lower);
    name.starts_with(".env")
        || matches!(name, ".npmrc" | ".pypirc" | ".netrc")
        || name.starts_with("id_rsa")
        || name.starts_with("id_ed25519")
        || name.starts_with("id_ecdsa")
        || name.starts_with("credentials")
        || matches!(name, "secret" | "secrets")
        || name.starts_with("secret.")
        || name.starts_with("secrets.")
        || [
            ".pem",
            ".key",
            ".p12",
            ".pfx",
            ".jks",
            ".keystore",
            ".kdbx",
            ".ppk",
        ]
        .iter()
        .any(|ext| name.ends_with(ext))
        || [".ssh/", ".aws/", ".gnupg/", ".kube/"]
            .iter()
            .any(|folder| lower.starts_with(folder) || lower.contains(&format!("/{folder}")))
}

/// Counts the words of an added line that suggest a bug fix.
pub fn fix_words(line: &str) -> u32 {
    line.split(|c: char| !c.is_alphanumeric())
        .filter(|word| {
            matches!(
                word.to_lowercase().as_str(),
                "fix"
                    | "fixes"
                    | "fixed"
                    | "bugfix"
                    | "hotfix"
                    | "bug"
                    | "bugs"
                    | "corrige"
                    | "arregla"
                    | "workaround"
                    | "regression"
                    | "regresion"
                    | "regresión"
            )
        })
        .count() as u32
}
