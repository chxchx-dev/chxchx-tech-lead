//! Links and codes in what an agent writes, made easy to use: bare URLs become clickable, and the
//! links and one-time codes (a device login code, a PIN) of a message are offered as buttons to open
//! or copy. Pure text rules, so they can be tested; opening and copying live in the interface.
//!
//! An agent's text is not trusted: only `http` and `https` links are ever opened, and a link is
//! never opened without the person clicking it.

/// Something a message offers to do.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    /// Open this link in the browser.
    Link { url: String, label: String },
    /// Copy this code.
    Code { value: String },
}

const MAX_LINKS: usize = 4;
const MAX_CODES: usize = 3;

fn is_url_char(c: char) -> bool {
    !c.is_whitespace() && !matches!(c, '<' | '>' | '"' | '\'' | '`' | '\u{201c}' | '\u{201d}')
}

/// Where each `http(s)://` link of `line` starts and ends, without the punctuation that closes a sentence.
fn urls_in(line: &str) -> Vec<(usize, usize)> {
    let mut found = Vec::new();
    let mut from = 0;
    while let Some(at) = line[from..].find("http") {
        let start = from + at;
        from = start + 4;
        let rest = &line[start..];
        if !(rest.starts_with("http://") || rest.starts_with("https://")) {
            continue;
        }
        // A URL starts a word: not the tail of `xhttps://…`.
        if line[..start]
            .chars()
            .next_back()
            .is_some_and(char::is_alphanumeric)
        {
            continue;
        }
        let mut end = start + rest.find(|c: char| !is_url_char(c)).unwrap_or(rest.len());
        loop {
            let url = &line[start..end];
            let Some(last) = url.chars().next_back() else {
                break;
            };
            let open = url.matches('(').count();
            let close = url.matches(')').count();
            let trailing = matches!(last, '.' | ',' | ';' | ':' | '!' | '?' | ']' | '}' | '*')
                || (last == ')' && close > open);
            if trailing {
                end -= last.len_utf8();
            } else {
                break;
            }
        }
        if end > start + "https://".len() - 1 {
            found.push((start, end));
            from = end;
        }
    }
    found
}

/// The text with every bare link between `<…>` so Markdown makes it clickable. Code blocks, inline
/// code and links that are already links are left as they are.
pub fn linkify(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 16);
    let mut fenced = false;
    for line in text.split_inclusive('\n') {
        if line.trim_start().starts_with("```") || line.trim_start().starts_with("~~~") {
            fenced = !fenced;
            out.push_str(line);
            continue;
        }
        if fenced || !line.contains("http") {
            out.push_str(line);
            continue;
        }
        let mut last = 0;
        for (start, end) in urls_in(line) {
            let before = &line[..start];
            let inside_code = before.matches('`').count() % 2 == 1;
            let already = before.ends_with('<') || before.ends_with('(') || before.ends_with('[');
            if inside_code || already {
                continue;
            }
            out.push_str(&line[last..start]);
            out.push('<');
            out.push_str(&line[start..end]);
            out.push('>');
            last = end;
        }
        out.push_str(&line[last..]);
    }
    out
}

/// The link if it is safe to open from here: `http` or `https`, with a host, no spaces or control
/// characters and of a sane length. Anything else (`file:`, `javascript:`, custom schemes) is refused.
pub fn safe_url(url: &str) -> Option<String> {
    let url = url.trim();
    let rest = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))?;
    let host = rest.split(['/', '?', '#']).next().unwrap_or("");
    let host = host.rsplit('@').next().unwrap_or(host);
    let host = host.split(':').next().unwrap_or(host);
    (url.len() <= 2_000
        && !host.is_empty()
        && host
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-'))
        && !url.chars().any(|c| c.is_whitespace() || c.is_control()))
    .then(|| url.to_owned())
}

/// A short, readable name for a link: its host and the start of its path.
pub fn label(url: &str) -> String {
    let rest = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .unwrap_or(url);
    let rest = rest.trim_end_matches('/');
    if rest.chars().count() <= 38 {
        rest.to_owned()
    } else {
        format!("{}…", rest.chars().take(37).collect::<String>())
    }
}

const CODE_WORDS: &[&str] = &[
    "code",
    "código",
    "codigo",
    "pin",
    "otp",
    "verification",
    "verificación",
    "verificacion",
    "device",
    "dispositivo",
    "user code",
    "ingresa",
    "introduce",
    "enter",
];

/// `ABCD-1234`: groups of capitals and digits joined by hyphens, the shape of a device login code.
fn device_code(token: &str) -> bool {
    let groups: Vec<&str> = token.split('-').collect();
    (2..=3).contains(&groups.len())
        && groups.iter().all(|group| {
            (3..=6).contains(&group.len())
                && group
                    .chars()
                    .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
        })
        && token.chars().any(|c| c.is_ascii_alphanumeric())
}

/// Codes mentioned on `line`: a device-code shape, or something in backticks that is short and has no
/// spaces. Only lines that talk about a code count, so a random `SHA-256` is not offered.
fn codes_in(line: &str) -> Vec<String> {
    let lower = line.to_lowercase();
    if !CODE_WORDS.iter().any(|word| lower.contains(word)) {
        return Vec::new();
    }
    let mut found = Vec::new();
    for token in line.split(|c: char| {
        c.is_whitespace() || matches!(c, ',' | ';' | ':' | '(' | ')' | '*' | '"' | '`' | '.')
    }) {
        if device_code(token) {
            found.push(token.to_owned());
        }
    }
    for (index, part) in line.split('`').enumerate() {
        let plausible = index % 2 == 1
            && (4..=32).contains(&part.chars().count())
            && !part.chars().any(char::is_whitespace)
            && !part.contains(['/', '\\', '=', '(', ')', '{', '}'])
            && !part.starts_with("--")
            && !part.contains("://")
            && part
                .chars()
                .any(|c| c.is_ascii_digit() || c.is_ascii_uppercase());
        if plausible {
            found.push(part.to_owned());
        }
    }
    found
}

/// The links and codes of a message, without repeats: what its buttons offer.
pub fn actions(text: &str) -> Vec<Action> {
    let mut links: Vec<Action> = Vec::new();
    let mut codes: Vec<Action> = Vec::new();
    for line in text.lines() {
        if line.contains("http") {
            for (start, end) in urls_in(line) {
                if let Some(url) = safe_url(&line[start..end]) {
                    let action = Action::Link {
                        label: label(&url),
                        url,
                    };
                    if !links.contains(&action) && links.len() < MAX_LINKS {
                        links.push(action);
                    }
                }
            }
        }
        for value in codes_in(line) {
            let action = Action::Code { value };
            if !codes.contains(&action) && codes.len() < MAX_CODES {
                codes.push(action);
            }
        }
    }
    links.extend(codes);
    links
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bare_links_become_clickable_and_everything_else_is_left_alone() {
        let text = "Abre https://auth.example.com/device y pon el código.\n\
                    Ya es un enlace: [aquí](https://a.dev/x) y <https://b.dev>.\n\
                    En código: `curl https://c.dev` no se toca.\n\
                    ```\nhttps://d.dev/en-un-bloque\n```\n\
                    Fin (ver https://e.dev/wiki/Rust_(lenguaje)).";
        let out = linkify(text);
        assert!(
            out.contains("<https://auth.example.com/device> y pon"),
            "{out}"
        );
        assert!(out.contains("[aquí](https://a.dev/x)") && out.contains("<https://b.dev>."));
        assert!(!out.contains("<https://c.dev"), "{out}");
        assert!(out.contains("\nhttps://d.dev/en-un-bloque\n"), "{out}");
        assert!(
            out.contains("<https://e.dev/wiki/Rust_(lenguaje)>)."),
            "el paréntesis de la dirección se queda: {out}"
        );
        assert_eq!(linkify("sin enlaces\nni nada"), "sin enlaces\nni nada");
        assert_eq!(linkify("xhttps://no.es/enlace"), "xhttps://no.es/enlace");
    }

    #[test]
    fn only_http_and_https_with_a_real_host_are_opened() {
        assert!(safe_url("https://github.com/login/device").is_some());
        assert!(safe_url("http://localhost:8080/cb?x=1#y").is_some());
        for bad in [
            "file:///etc/passwd",
            "javascript:alert(1)",
            "ftp://x.dev",
            "https://",
            "https:///solo-ruta",
            "https://exa mple.com",
            "https://evil.com\n.otro",
            "mailto:a@b.c",
            "https://ex_ample.com/",
        ] {
            assert!(safe_url(bad).is_none(), "{bad}");
        }
        assert!(safe_url(&format!("https://a.dev/{}", "x".repeat(2_100))).is_none());
        assert_eq!(
            label("https://github.com/login/device/"),
            "github.com/login/device"
        );
        assert!(label(&format!("https://a.dev/{}", "x".repeat(100))).ends_with('…'));
    }

    #[test]
    fn a_login_message_offers_its_link_and_its_code() {
        let text = "Para entrar:\n1. Abre https://github.com/login/device\n2. Ingresa el código ABCD-1234 y confirma.\n\
                    Si no, usa https://github.com/login/device otra vez.";
        let found = actions(text);
        assert_eq!(
            found,
            vec![
                Action::Link {
                    url: "https://github.com/login/device".into(),
                    label: "github.com/login/device".into()
                },
                Action::Code {
                    value: "ABCD-1234".into()
                },
            ],
            "sin repetir el enlace"
        );
        let pin = actions("Tu PIN es `482913`, no lo compartas.");
        assert_eq!(
            pin,
            vec![Action::Code {
                value: "482913".into()
            }]
        );
    }

    #[test]
    fn a_code_is_only_offered_where_the_text_talks_about_one() {
        assert!(actions("El hash SHA-256 y ISO-8601 salen en la tabla.").is_empty());
        assert!(actions("Corre `cargo test --locked` y mira `src/lib.rs`.").is_empty());
        assert!(
            actions("El código de salida es `1`.").is_empty(),
            "demasiado corto para ser un código"
        );
        let many = (0..10)
            .map(|n| format!("Abre https://a{n}.dev/x"))
            .collect::<Vec<_>>()
            .join("\n");
        assert_eq!(actions(&many).len(), MAX_LINKS);
    }
}
