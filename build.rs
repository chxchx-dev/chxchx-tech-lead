//! Build-time packaging: Windows executable metadata and the bundled catalog.
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    embed_windows_resources();
    bundle_catalog();
}

fn embed_windows_resources() {
    println!("cargo:rerun-if-changed=assets/brand/chxchx.ico");
    println!("cargo:rerun-if-changed=assets/brand/chxchx.manifest");
    let windows = std::env::var("CARGO_CFG_TARGET_OS").is_ok_and(|os| os == "windows");
    if !windows {
        return;
    }

    let mut binaries = Vec::new();
    if std::env::var_os("CARGO_FEATURE_CLI").is_some() {
        binaries.extend([("chxchx-tech", "ChxChx Tech"), ("chxchx", "ChxChx")]);
    }
    if std::env::var_os("CARGO_FEATURE_STUDIO").is_some() {
        binaries.push(("chxchx-studio", "ChxChx Studio"));
    }
    if std::env::var_os("CARGO_FEATURE_PYTHON_HELPER_LAUNCHER").is_some() {
        binaries.push(("chxchx-python-helper", "ChxChx Python Helper"));
    }

    let version = std::env::var("CARGO_PKG_VERSION").expect("Cargo sets CARGO_PKG_VERSION");
    let major = std::env::var("CARGO_PKG_VERSION_MAJOR")
        .expect("Cargo sets CARGO_PKG_VERSION_MAJOR")
        .parse::<u16>()
        .expect("Windows file version major must fit in 16 bits");
    let minor = std::env::var("CARGO_PKG_VERSION_MINOR")
        .expect("Cargo sets CARGO_PKG_VERSION_MINOR")
        .parse::<u16>()
        .expect("Windows file version minor must fit in 16 bits");
    let patch = std::env::var("CARGO_PKG_VERSION_PATCH")
        .expect("Cargo sets CARGO_PKG_VERSION_PATCH")
        .parse::<u16>()
        .expect("Windows file version patch must fit in 16 bits");
    let output = PathBuf::from(std::env::var_os("OUT_DIR").expect("OUT_DIR"));
    let manifest = std::path::absolute("assets/brand/chxchx.manifest")
        .expect("absolute Windows application manifest");

    for (binary, description) in binaries {
        let manifest_path = manifest.to_string_lossy().replace('\\', "/");
        let icon = (binary == "chxchx-studio")
            .then(|| std::path::absolute("assets/brand/chxchx.ico").expect("absolute icon"))
            .map(|path| format!("1 ICON \"{}\"\n", path.to_string_lossy().replace('\\', "/")))
            .unwrap_or_default();
        let resource = format!(
            r#"1 24 "{manifest_path}"
#include <windows.h>
{icon}VS_VERSION_INFO VERSIONINFO
 FILEVERSION {major},{minor},{patch},0
 PRODUCTVERSION {major},{minor},{patch},0
 FILEFLAGSMASK 0x3fL
 FILEFLAGS 0x0L
 FILEOS VOS_NT_WINDOWS32
 FILETYPE VFT_APP
 FILESUBTYPE 0x0L
BEGIN
  BLOCK "StringFileInfo"
  BEGIN
    BLOCK "040904B0"
    BEGIN
      VALUE "CompanyName", "ChxChx\0"
      VALUE "FileDescription", "{description}\0"
      VALUE "FileVersion", "{version}\0"
      VALUE "InternalName", "{binary}\0"
      VALUE "OriginalFilename", "{binary}.exe\0"
      VALUE "ProductName", "ChxChx\0"
      VALUE "ProductVersion", "{version}\0"
    END
  END
  BLOCK "VarFileInfo"
  BEGIN
    VALUE "Translation", 0x409, 1200
  END
END
"#
        );
        let resource_path = output.join(format!("{binary}.rc"));
        fs::write(&resource_path, resource).expect("write Windows version resource");

        embed_resource::compile_for(&resource_path, [binary], embed_resource::NONE)
            .manifest_required()
            .unwrap_or_else(|error| {
                panic!("No se pudieron incrustar los recursos de Windows en {binary}: {error}")
            });
    }
}

/// The catalog (skills, packs and mods) is embedded in the binary from `catalog/`.
const CATALOG: &str = "catalog";

fn bundle_catalog() {
    let base = Path::new(CATALOG);
    let skills = base.join("skills");
    let packs = base.join("packs");
    let mods = base.join("mods");
    let connectors = base.join("connectors");
    for path in [&skills, &packs, &mods, &connectors] {
        println!("cargo:rerun-if-changed={}", path.display());
    }
    let mut out = String::new();
    write_entries(&mut out, "SKILLS", &skills, "skill.toml", true);
    write_entries(&mut out, "MODS", &mods, "mod.toml", true);
    write_entries(&mut out, "PACKS", &packs, "", false);
    write_entries(&mut out, "CONNECTORS", &connectors, "", false);
    let target =
        PathBuf::from(std::env::var_os("OUT_DIR").expect("OUT_DIR")).join("catalog_data.rs");
    fs::write(target, out).expect("write catalog_data.rs");
}

/// Emits `const NAME: &[(&str, &str, &str)]` of (id, manifest, instructions).
fn write_entries(out: &mut String, name: &str, root: &Path, manifest: &str, folders: bool) {
    let mut entries: Vec<PathBuf> = fs::read_dir(root)
        .map(|read| {
            read.filter_map(Result::ok)
                .map(|entry| entry.path())
                .collect()
        })
        .unwrap_or_default();
    entries.sort();
    if entries.is_empty() {
        println!(
            "cargo:warning=Catálogo vacío o ausente en {}",
            root.display()
        );
    }
    let _ = writeln!(out, "pub(super) const {name}: &[(&str, &str, &str)] = &[");
    for entry in entries {
        let (id, manifest_path, instructions) = if folders {
            if !entry.is_dir() {
                continue;
            }
            let id = entry
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            (id, entry.join(manifest), Some(entry.join("SKILL.md")))
        } else {
            if entry
                .extension()
                .is_none_or(|extension| extension != "toml")
            {
                continue;
            }
            let id = entry
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            (id, entry.clone(), None)
        };
        let Ok(manifest_path) = std::path::absolute(&manifest_path) else {
            continue;
        };
        println!("cargo:rerun-if-changed={}", manifest_path.display());
        let instructions = match instructions {
            Some(path) => {
                let path = std::path::absolute(path).unwrap_or_default();
                println!("cargo:rerun-if-changed={}", path.display());
                format!("include_str!({:?})", path.to_string_lossy())
            }
            None => "\"\"".to_owned(),
        };
        let _ = writeln!(
            out,
            "    ({id:?}, include_str!({:?}), {instructions}),",
            manifest_path.to_string_lossy()
        );
    }
    let _ = writeln!(out, "];");
}
