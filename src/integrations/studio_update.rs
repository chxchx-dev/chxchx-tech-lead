//! Studio's own updates: tells the person when a newer release is published and opens its page.
//!
//! The product is installed by its installers (and the MSI, DMG and archives of each release), so
//! this does not replace binaries itself: it reads which release is the latest and sends the
//! person to it. The latest release is the one `releases/latest` redirects to, read with the
//! system's `curl` (HTTPS only, short timeout), so no account, token or API quota is involved.
use super::chat_bridge::run_short;
use super::executable_search::agent_path;
use std::cmp::Ordering;
use std::process::Command;
use std::time::Duration;

pub const OWNER: &str = "chxchx-dev";
pub const REPOSITORY: &str = "chxchx-tech-lead";

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Info {
    /// The version this program is.
    pub current: String,
    /// The latest published release, without the leading `v`.
    pub latest: String,
    pub available: bool,
    /// The page of that release.
    pub url: String,
}

fn releases_url() -> String {
    format!("https://github.com/{OWNER}/{REPOSITORY}/releases")
}

/// The tag a `releases/latest` redirect points at: `…/releases/tag/v0.6.3` → `v0.6.3`.
pub fn parse_tag(redirect: &str) -> Result<String, String> {
    let tag = redirect
        .trim()
        .split_once("/releases/tag/")
        .map(|(_, tag)| tag.trim_end_matches('/'))
        .filter(|tag| {
            !tag.is_empty()
                && tag.len() <= 64
                && tag
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | '+'))
        })
        .ok_or("Todavía no hay una versión publicada.")?;
    Ok(tag.to_owned())
}

/// `major.minor.patch` and whether a pre-release suffix follows (`0.6.0a1`, `1.2.3-rc.1`).
fn parts(version: &str) -> Option<((u64, u64, u64), String)> {
    let version = version.trim().trim_start_matches('v');
    let numeric = version
        .find(|c: char| !c.is_ascii_digit() && c != '.')
        .unwrap_or(version.len());
    let (numbers, suffix) = version.split_at(numeric);
    let mut fields = numbers.split('.');
    let mut next = || fields.next().and_then(|field| field.parse::<u64>().ok());
    let triple = (next()?, next()?, next()?);
    (fields.next().is_none()).then(|| (triple, suffix.trim_start_matches('-').to_owned()))
}

/// Whether `latest` is a newer version than `current`. A pre-release is older than its release.
pub fn is_newer(latest: &str, current: &str) -> bool {
    let (Some((a, a_pre)), Some((b, b_pre))) = (parts(latest), parts(current)) else {
        return false;
    };
    match a.cmp(&b) {
        Ordering::Greater => true,
        Ordering::Less => false,
        Ordering::Equal => match (a_pre.is_empty(), b_pre.is_empty()) {
            (true, false) => true,
            (false, true) => false,
            (true, true) => false,
            (false, false) => a_pre > b_pre,
        },
    }
}

/// What the check says, given where the latest release's tag came from. Tests give it directly.
pub fn check_with(
    current: &str,
    latest_tag: impl FnOnce() -> Result<String, String>,
) -> Result<Info, String> {
    let tag = latest_tag()?;
    let latest = tag.trim_start_matches('v').to_owned();
    Ok(Info {
        current: current.to_owned(),
        available: is_newer(&latest, current),
        url: format!("{}/tag/{tag}", releases_url()),
        latest,
    })
}

/// The tag of the latest release, read from the redirect of `releases/latest`. Blocking.
fn latest_tag() -> Result<String, String> {
    let null = if cfg!(windows) { "NUL" } else { "/dev/null" };
    let mut curl = Command::new("curl");
    curl.env("PATH", agent_path()).args([
        "--head",
        "--silent",
        "--show-error",
        "--proto",
        "=https",
        "--max-time",
        "15",
        "--output",
        null,
        "--write-out",
        "%{redirect_url}",
        &format!("{}/latest", releases_url()),
    ]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        curl.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    let (code, output) = run_short(curl, Duration::from_secs(25)).map_err(|_| {
        "No se pudo consultar GitHub (¿hay conexión? ¿está instalado curl?).".to_owned()
    })?;
    if code != 0 {
        return Err("No se pudo consultar GitHub; revisa tu conexión.".to_owned());
    }
    parse_tag(&output)
}

/// Whether a newer release is published. Blocking: run it off the UI thread.
pub fn check() -> Result<Info, String> {
    check_with(env!("CARGO_PKG_VERSION"), latest_tag)
}

/// Opens the release page in the person's browser. Only pages of this project's releases open.
pub fn open_download(info: &Info) -> Result<(), String> {
    if !info.url.starts_with(&format!("{}/", releases_url())) {
        return Err("La dirección de la descarga no es de los releases del proyecto.".to_owned());
    }
    let mut opener = if cfg!(target_os = "macos") {
        Command::new("open")
    } else if cfg!(windows) {
        let mut command = Command::new("rundll32");
        command.arg("url.dll,FileProtocolHandler");
        command
    } else {
        Command::new("xdg-open")
    };
    opener
        .arg(&info.url)
        .env("PATH", agent_path())
        .spawn()
        .map(|_| ())
        .map_err(|error| {
            format!(
                "No pude abrir el navegador: {error}. Abre {} a mano.",
                info.url
            )
        })
}

#[cfg(test)]
#[path = "studio_update_tests.rs"]
mod tests;
