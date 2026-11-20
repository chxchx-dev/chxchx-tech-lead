use super::*;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("chxchx-errors-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn credentials_are_replaced_and_the_rest_of_the_message_is_kept() {
    for (input, expected) in [
        (
            "falló con api_key=abc123 y siguió",
            "falló con api_key=[REDACTED] y siguió",
        ),
        (
            "password: hunter2, reintentando",
            "password: [REDACTED], reintentando",
        ),
        (
            "Authorization: Bearer abc.def-ghi== listo",
            "Authorization: [REDACTED] listo",
        ),
        ("usé Bearer abc.def_ghi", "usé Bearer [REDACTED]"),
        (
            "clave sk-abcdefghijklmnop1234 filtrada",
            "clave [REDACTED] filtrada",
        ),
        (
            "ghp_abcdefghijklmnopqrstu1234 en el log",
            "[REDACTED] en el log",
        ),
        ("TOKEN = secreto;fin", "TOKEN = [REDACTED];fin"),
    ] {
        assert_eq!(redact(input), expected, "{input}");
    }
}

#[test]
fn ordinary_text_is_left_alone() {
    for text in [
        "no se pudo abrir el archivo",
        "tokenizer cargado",      // `token` inside a word is not an assignment
        "el password es largo",   // no `:` or `=` follows
        "sk-corto y ghx_ninguno", // too short, or not a token shape
        "información: acentuación y ñ",
    ] {
        assert_eq!(redact(text), text);
    }
}

#[test]
fn entries_are_listed_newest_first_and_can_be_scoped_to_a_project() {
    let home = scratch("list");
    let (one, two) = (home.join("uno"), home.join("dos"));
    fs::create_dir_all(&one).unwrap();
    fs::create_dir_all(&two).unwrap();
    record(&home, "uno", &one, "start", "primero").unwrap();
    record(&home, "dos", &two, "stop", "segundo").unwrap();
    record(&home, "uno", &one, "start", "tercero").unwrap();

    let all = list(&home, None, 10);
    assert_eq!(
        all.iter().map(|e| e.message.as_str()).collect::<Vec<_>>(),
        ["tercero", "segundo", "primero"]
    );
    let only_one = list(&home, Some(&one), 10);
    assert_eq!(
        only_one
            .iter()
            .map(|e| e.message.as_str())
            .collect::<Vec<_>>(),
        ["tercero", "primero"]
    );
    assert_eq!(list(&home, None, 1).len(), 1);
    assert!(list(&home, Some(&home.join("nadie")), 10).is_empty());
}

#[test]
fn the_cache_keeps_the_newest_two_hundred_and_clips_long_messages() {
    let home = scratch("bounds");
    for index in 0..MAX_ENTRIES + 5 {
        record(&home, "p", &home, "op", &format!("error {index}")).unwrap();
    }
    let all = list(&home, None, usize::MAX);
    assert_eq!(all.len(), MAX_ENTRIES);
    assert_eq!(all[0].message, format!("error {}", MAX_ENTRIES + 4));
    assert_eq!(all.last().unwrap().message, "error 5");

    record(&home, "p", &home, "op", &"x".repeat(10_000)).unwrap();
    assert_eq!(list(&home, None, 1)[0].message.chars().count(), 2400);
}

#[test]
fn what_is_recorded_has_no_secrets_and_damaged_lines_are_skipped() {
    let home = scratch("safe");
    record(&home, "p", &home, "login", "token=abcdef falló").unwrap();
    let path = cache_path(&home);
    let mut text = fs::read_to_string(&path).unwrap();
    assert!(!text.contains("abcdef"));
    text.push_str("esto no es json\n");
    fs::write(&path, text).unwrap();

    assert_eq!(
        list(&home, None, 10).len(),
        1,
        "a damaged line does not hide the rest"
    );
}

#[cfg(unix)]
#[test]
fn a_symlinked_cache_is_never_written_through() {
    let home = scratch("link");
    let outside = scratch("link-outside").join("target.jsonl");
    fs::create_dir_all(home.join("cache")).unwrap();
    std::os::unix::fs::symlink(&outside, cache_path(&home)).unwrap();

    assert!(
        record(&home, "p", &home, "op", "mensaje")
            .unwrap_err()
            .contains("enlace simbólico")
    );
    assert!(!outside.exists());
}
