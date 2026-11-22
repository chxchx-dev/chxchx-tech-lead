//! Applies a signed release on top of this installation (ADR 0017, H6).
//!
//! Order matters and every step can stop the whole thing before anything is replaced: the manifest's
//! signature is checked, the package's SHA-256 is checked against the signed manifest, the archive is
//! inspected before it is unpacked, the new program proves it runs, running jobs are given a chance
//! to finish, and only then are files swapped (keeping the old ones). If the installed program does
//! not answer afterwards, the old files are put back. Only Linux archives are applied here; the
//! other systems keep using their installers.

use crate::features::release_manifest::{self as release, Manifest};
use std::path::{Path, PathBuf};
use std::process::Command;

pub const MANIFEST: &str = "studio-manifest.txt";
pub const SIGNATURE: &str = "studio-manifest.txt.sig";

/// Where release files come from.
pub trait Source {
    fn file(&self, name: &str, to: &Path) -> Result<(), String>;
}

/// The project's GitHub releases, read with the system's `curl` over HTTPS only.
pub struct GitHub {
    pub dev: bool,
}

impl Source for GitHub {
    fn file(&self, name: &str, to: &Path) -> Result<(), String> {
        use super::studio_update::{OWNER, REPOSITORY};
        let base = format!("https://github.com/{OWNER}/{REPOSITORY}/releases");
        let url = if self.dev {
            format!("{base}/download/studio-dev-preview/{name}")
        } else {
            format!("{base}/latest/download/{name}")
        };
        let mut curl = Command::new("curl");
        curl.env("PATH", super::executable_search::agent_path())
            .args([
                "--fail",
                "--location",
                "--silent",
                "--show-error",
                "--proto",
                "=https",
                "--proto-redir",
                "=https",
                "--max-time",
                "900",
                "--output",
            ]);
        curl.arg(to).arg(&url);
        let output = curl
            .output()
            .map_err(|_| "No se pudo ejecutar curl (¿está instalado?).".to_owned())?;
        if output.status.success() {
            Ok(())
        } else {
            Err(format!(
                "No se pudo descargar {name}: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ))
        }
    }
}

#[derive(Debug)]
pub struct Offer {
    pub manifest: Manifest,
    pub file: String,
    pub sha256: String,
}

fn read(source: &dyn Source, work: &Path, name: &str) -> Result<Vec<u8>, String> {
    let target = work.join(name);
    source.file(name, &target)?;
    std::fs::read(&target).map_err(|error| format!("{name}: {error}"))
}

/// The signed release that is newer than `current`, or `None` when this is the latest.
/// With no key the answer is an error: an unsigned release is never trusted.
pub fn check(
    source: &dyn Source,
    work: &Path,
    current: &str,
    key: Option<[u8; 32]>,
) -> Result<Option<Offer>, String> {
    let key = key.ok_or(
        "Esta versión no trae clave de firma, así que no aplica actualizaciones por sí sola. \
         Descarga el instalador de la página de lanzamientos.",
    )?;
    std::fs::create_dir_all(work).map_err(|error| format!("{}: {error}", work.display()))?;
    let manifest = read(source, work, MANIFEST)?;
    let signature = String::from_utf8_lossy(&read(source, work, SIGNATURE)?).into_owned();
    release::verify(&manifest, &signature, &key)?;
    let manifest = parse_utf8(&manifest)?;
    if !super::studio_update::is_newer(&manifest.version, current) {
        return Ok(None);
    }
    let (_, file, sha256) = manifest
        .linux_package()
        .cloned()
        .ok_or("Este lanzamiento no trae un paquete de Linux que se pueda aplicar.")?;
    Ok(Some(Offer {
        manifest,
        file,
        sha256,
    }))
}

fn parse_utf8(bytes: &[u8]) -> Result<Manifest, String> {
    release::parse(&String::from_utf8_lossy(bytes))
}

fn run_ok(program: &Path, args: &[&str]) -> Result<String, String> {
    let output = Command::new(program)
        .args(args)
        .output()
        .map_err(|error| format!("{}: {error}", program.display()))?;
    if !output.status.success() {
        return Err(format!("{} terminó con error.", program.display()));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Refuses archives that could write outside their folder or contain links.
fn inspect_archive(archive: &Path) -> Result<(), String> {
    let names = Command::new("tar")
        .arg("-tzf")
        .arg(archive)
        .output()
        .map_err(|_| "No se pudo ejecutar tar.".to_owned())?;
    let kinds = Command::new("tar")
        .arg("-tvzf")
        .arg(archive)
        .output()
        .map_err(|_| "No se pudo ejecutar tar.".to_owned())?;
    if !names.status.success() || !kinds.status.success() {
        return Err("El paquete descargado no es un archivo válido.".to_owned());
    }
    for name in String::from_utf8_lossy(&names.stdout).lines() {
        if name.starts_with('/') || name.split('/').any(|part| part == "..") {
            return Err(format!("El paquete trae una ruta peligrosa: {name}"));
        }
    }
    for line in String::from_utf8_lossy(&kinds.stdout).lines() {
        if !matches!(line.chars().next(), Some('-' | 'd')) {
            return Err("El paquete trae enlaces o archivos especiales: no se aplica.".to_owned());
        }
    }
    Ok(())
}

/// Whether this program can apply the package a release carries: only Linux on x86_64. On the
/// other systems the installers (MSI, DMG, `.tar.gz`) are the way to update, and applying a Linux
/// package there would only fail late and confusingly.
pub fn platform_supported(os: &str, arch: &str) -> Result<(), String> {
    if os == "linux" && arch == "x86_64" {
        return Ok(());
    }
    Err(format!(
        "Aplicar la actualización por sí solo solo está disponible en Linux x86_64 (esto es {os} {arch}). \
         Descarga el instalador de tu sistema de la página de lanzamientos."
    ))
}

/// Where a release is applied: the folder of the running program, which must be an installation.
pub fn install_dir() -> Result<PathBuf, String> {
    let exe = std::env::current_exe().map_err(|error| error.to_string())?;
    let dir = exe
        .parent()
        .ok_or("No se sabe dónde está instalado el programa.")?;
    if dir
        .ancestors()
        .any(|part| part.file_name().is_some_and(|name| name == "target"))
    {
        return Err(
            "Esto es una compilación de desarrollo, no una instalación: no se actualiza."
                .to_owned(),
        );
    }
    Ok(dir.to_path_buf())
}

/// The installed program answers with the version it should.
pub fn answers_with(dir: &Path, version: &str) -> bool {
    run_ok(&dir.join("chxchx-tech"), &["version"]).is_ok_and(|text| text.contains(version))
}

/// Downloads, checks and swaps in the release. `drain` is called right before the swap.
/// `installed_ok` decides whether the swap worked; the real one is `answers_with`.
pub fn apply(
    offer: &Offer,
    source: &dyn Source,
    install: &Path,
    work: &Path,
    drain: &dyn Fn() -> Result<(), String>,
    installed_ok: &dyn Fn(&Path, &str) -> bool,
) -> Result<Vec<String>, String> {
    let archive = work.join(&offer.file);
    source.file(&offer.file, &archive)?;
    let bytes =
        std::fs::read(&archive).map_err(|error| format!("{}: {error}", archive.display()))?;
    if release::sha256_hex(&bytes) != offer.sha256 {
        return Err("El paquete no coincide con el SHA-256 firmado: se descartó.".to_owned());
    }
    inspect_archive(&archive)?;
    let unpacked = work.join("unpacked");
    let _ = std::fs::remove_dir_all(&unpacked);
    std::fs::create_dir_all(&unpacked).map_err(|error| error.to_string())?;
    let status = Command::new("tar")
        .args(["-xzf"])
        .arg(&archive)
        .arg("-C")
        .arg(&unpacked)
        .arg("--no-same-owner")
        .status()
        .map_err(|_| "No se pudo ejecutar tar.".to_owned())?;
    if !status.success() {
        return Err("No se pudo desempaquetar la actualización.".to_owned());
    }
    let new_bin = find_bin(&unpacked).ok_or("El paquete no trae la carpeta bin/.")?;
    if !run_ok(&new_bin.join("chxchx-tech"), &["version"])
        .is_ok_and(|text| text.contains(&offer.manifest.version))
    {
        return Err(
            "El programa nuevo no responde con la versión anunciada: no se aplica.".to_owned(),
        );
    }
    drain()?;
    swap(&new_bin, install, &offer.manifest.version, installed_ok)
}

fn find_bin(root: &Path) -> Option<PathBuf> {
    std::fs::read_dir(root)
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path().join("bin"))
        .find(|bin| bin.join("chxchx-tech").is_file())
}

fn swap(
    new_bin: &Path,
    install: &Path,
    version: &str,
    installed_ok: &dyn Fn(&Path, &str) -> bool,
) -> Result<Vec<String>, String> {
    let backup = install.join(".update-backup");
    let _ = std::fs::remove_dir_all(&backup);
    std::fs::create_dir_all(&backup).map_err(|error| format!("{}: {error}", backup.display()))?;
    let mut names: Vec<String> = std::fs::read_dir(new_bin)
        .map_err(|error| error.to_string())?
        .filter_map(Result::ok)
        .filter(|entry| entry.path().is_file())
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| install.join(name).is_file())
        .collect();
    names.sort();
    let mut moved: Vec<String> = Vec::new();
    let result = (|| -> Result<(), String> {
        for name in &names {
            let staged = install.join(format!(".{name}.new"));
            std::fs::copy(new_bin.join(name), &staged)
                .map_err(|error| format!("{name}: {error}"))?;
            std::fs::rename(install.join(name), backup.join(name))
                .map_err(|error| format!("{name}: {error}"))?;
            moved.push(name.clone());
            std::fs::rename(&staged, install.join(name))
                .map_err(|error| format!("{name}: {error}"))?;
        }
        if installed_ok(install, version) {
            Ok(())
        } else {
            Err("El programa instalado no responde tras la actualización.".to_owned())
        }
    })();
    match result {
        Ok(()) => Ok(names
            .iter()
            .map(|name| format!("Actualizado {name}"))
            .collect()),
        Err(error) => {
            for name in &moved {
                let _ = std::fs::rename(backup.join(name), install.join(name));
            }
            Err(format!("{error} Se restauraron los archivos anteriores."))
        }
    }
}

#[cfg(test)]
mod platform_tests {
    use super::platform_supported;

    #[test]
    fn only_linux_x86_64_applies_updates_by_itself() {
        assert!(platform_supported("linux", "x86_64").is_ok());
        for (os, arch) in [
            ("windows", "x86_64"),
            ("macos", "aarch64"),
            ("macos", "x86_64"),
            ("linux", "aarch64"),
        ] {
            let error = platform_supported(os, arch).unwrap_err();
            assert!(
                error.contains(os) && error.contains("instalador"),
                "{error}"
            );
        }
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};
    use std::os::unix::fs::PermissionsExt;

    struct Dir(PathBuf);
    impl Source for Dir {
        fn file(&self, name: &str, to: &Path) -> Result<(), String> {
            std::fs::copy(self.0.join(name), to)
                .map(|_| ())
                .map_err(|error| format!("{name}: {error}"))
        }
    }

    fn script(path: &Path, version: &str) {
        std::fs::write(path, format!("#!/bin/sh\necho \"chxchx-tech {version}\"\n")).unwrap();
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }

    struct World {
        root: PathBuf,
        key: [u8; 32],
    }

    /// A release of version `new` served from a folder, and an installation of `0.6.3`.
    fn world(name: &str, new: &str, tamper: bool) -> World {
        let root = std::env::temp_dir().join(format!("chxchx-upd-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let pack = root.join("pack/chxchx-studio-linux/bin");
        std::fs::create_dir_all(&pack).unwrap();
        script(&pack.join("chxchx-tech"), new);
        script(&pack.join("chxchx"), new);
        let served = root.join("served");
        std::fs::create_dir_all(&served).unwrap();
        let archive = served.join("chxchx-studio-ubuntu-x86_64.tar.gz");
        assert!(
            Command::new("tar")
                .arg("-czf")
                .arg(&archive)
                .arg("-C")
                .arg(root.join("pack"))
                .arg("chxchx-studio-linux")
                .status()
                .unwrap()
                .success()
        );
        let digest = release::sha256_hex(&std::fs::read(&archive).unwrap());
        let manifest = format!(
            "schema_version=1\nchannel=stable\ntag=v{new}\ncommit={}\nversion={new}\nasset.linux-ubuntu-x86_64=chxchx-studio-ubuntu-x86_64.tar.gz\nsha256.linux-ubuntu-x86_64={digest}\n",
            "a".repeat(40)
        );
        let signing = SigningKey::from_bytes(&[5; 32]);
        std::fs::write(served.join(MANIFEST), &manifest).unwrap();
        std::fs::write(
            served.join(SIGNATURE),
            release::to_hex(&signing.sign(manifest.as_bytes()).to_bytes()),
        )
        .unwrap();
        if tamper {
            std::fs::write(&archive, b"not the signed package").unwrap();
        }
        let install = root.join("install");
        std::fs::create_dir_all(&install).unwrap();
        script(&install.join("chxchx-tech"), "0.6.3");
        script(&install.join("chxchx"), "0.6.3");
        World {
            root,
            key: signing.verifying_key().to_bytes(),
        }
    }

    fn run_update(w: &World, ok: &dyn Fn(&Path, &str) -> bool) -> Result<Vec<String>, String> {
        let source = Dir(w.root.join("served"));
        let work = w.root.join("work");
        let offer = check(&source, &work, "0.6.3", Some(w.key))?.ok_or("sin novedades")?;
        apply(
            &offer,
            &source,
            &w.root.join("install"),
            &work,
            &|| Ok(()),
            ok,
        )
    }

    #[test]
    fn a_signed_newer_release_replaces_the_files_and_keeps_the_old_ones() {
        let w = world("ok", "0.7.0", false);
        run_update(&w, &answers_with).unwrap();
        assert!(answers_with(&w.root.join("install"), "0.7.0"));
        let kept = run_ok(
            &w.root.join("install/.update-backup/chxchx-tech"),
            &["version"],
        )
        .unwrap();
        assert!(kept.contains("0.6.3"));
        let _ = std::fs::remove_dir_all(&w.root);
    }

    #[test]
    fn a_package_that_is_not_the_signed_one_changes_nothing() {
        let w = world("tamper", "0.7.0", true);
        let error = run_update(&w, &answers_with).unwrap_err();
        assert!(error.contains("SHA-256"), "{error}");
        assert!(answers_with(&w.root.join("install"), "0.6.3"));
        let _ = std::fs::remove_dir_all(&w.root);
    }

    #[test]
    fn a_manifest_signed_by_someone_else_or_with_no_key_is_refused() {
        let w = world("sig", "0.7.0", false);
        let source = Dir(w.root.join("served"));
        let work = w.root.join("work");
        assert!(check(&source, &work, "0.6.3", Some([9; 32])).is_err());
        assert!(
            check(&source, &work, "0.6.3", None)
                .unwrap_err()
                .contains("clave de firma")
        );
        let _ = std::fs::remove_dir_all(&w.root);
    }

    #[test]
    fn the_same_or_an_older_version_is_not_an_update() {
        let w = world("same", "0.6.3", false);
        let source = Dir(w.root.join("served"));
        assert!(
            check(&source, &w.root.join("work"), "0.6.3", Some(w.key))
                .unwrap()
                .is_none()
        );
        let _ = std::fs::remove_dir_all(&w.root);
    }

    #[test]
    fn if_the_installed_program_stops_answering_the_old_files_come_back() {
        let w = world("undo", "0.7.0", false);
        let error = run_update(&w, &|_, _| false).unwrap_err();
        assert!(error.contains("restauraron"), "{error}");
        assert!(answers_with(&w.root.join("install"), "0.6.3"));
        let _ = std::fs::remove_dir_all(&w.root);
    }

    #[test]
    fn running_jobs_stop_the_update_before_anything_is_replaced() {
        let w = world("busy", "0.7.0", false);
        let source = Dir(w.root.join("served"));
        let work = w.root.join("work");
        let offer = check(&source, &work, "0.6.3", Some(w.key))
            .unwrap()
            .unwrap();
        let busy = || Err::<(), String>("Hay 2 trabajos en curso.".to_owned());
        let error = apply(
            &offer,
            &source,
            &w.root.join("install"),
            &work,
            &busy,
            &answers_with,
        )
        .unwrap_err();
        assert!(error.contains("trabajos"));
        assert!(answers_with(&w.root.join("install"), "0.6.3"));
        let _ = std::fs::remove_dir_all(&w.root);
    }
}
