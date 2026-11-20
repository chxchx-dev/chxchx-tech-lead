use super::*;

#[test]
fn the_tag_comes_from_the_release_redirect() {
    let redirect = "https://github.com/chxchx-dev/chxchx-tech-lead/releases/tag/v0.6.3";
    assert_eq!(parse_tag(redirect).unwrap(), "v0.6.3");
    assert_eq!(parse_tag(&format!("{redirect}\n")).unwrap(), "v0.6.3");
    assert_eq!(parse_tag(&format!("{redirect}/")).unwrap(), "v0.6.3");
}

#[test]
fn no_release_or_a_strange_answer_is_not_a_tag() {
    for text in [
        "",
        "https://github.com/chxchx-dev/chxchx-tech-lead/releases",
        "https://github.com/x/y/releases/tag/",
        "https://github.com/x/y/releases/tag/v1; rm -rf",
        "https://github.com/x/y/releases/tag/v1 2",
    ] {
        assert!(parse_tag(text).is_err(), "{text:?}");
    }
}

#[test]
fn versions_compare_by_number_and_a_prerelease_is_older_than_its_release() {
    for (latest, current, newer) in [
        ("0.6.4", "0.6.3", true),
        ("v0.7.0", "0.6.9", true),
        ("1.0.0", "0.99.99", true),
        ("0.6.3", "0.6.3", false),
        ("0.6.2", "0.6.3", false),
        ("0.6.0", "0.6.0a1", true),
        ("0.6.0a1", "0.6.0", false),
        ("0.6.0a2", "0.6.0a1", true),
        ("0.10.0", "0.9.0", true),
        ("basura", "0.6.3", false),
        ("0.6", "0.6.3", false),
    ] {
        assert_eq!(is_newer(latest, current), newer, "{latest} vs {current}");
    }
}

#[test]
fn the_check_reports_what_is_published_and_where() {
    let info = check_with("0.6.3", || Ok("v0.7.0".into())).unwrap();
    assert!(info.available);
    assert_eq!(
        (info.current.as_str(), info.latest.as_str()),
        ("0.6.3", "0.7.0")
    );
    assert_eq!(
        info.url,
        "https://github.com/chxchx-dev/chxchx-tech-lead/releases/tag/v0.7.0"
    );

    let same = check_with("0.7.0", || Ok("v0.7.0".into())).unwrap();
    assert!(!same.available);
    assert_eq!(
        check_with("0.7.0", || Err("sin red".into())).unwrap_err(),
        "sin red"
    );
}

#[test]
fn only_release_pages_of_this_project_are_opened() {
    for url in [
        "https://evil.example/releases/tag/v1",
        "file:///etc/passwd",
        "https://github.com/otro/repo/releases/tag/v1",
        "",
    ] {
        let info = Info {
            url: url.into(),
            ..Info::default()
        };
        assert!(
            open_download(&info)
                .unwrap_err()
                .contains("no es de los releases"),
            "{url}"
        );
    }
}
