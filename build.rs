//! Build-time packaging: Windows executable metadata, the bundled catalog and (with the `node`
//! feature) the web client the node serves.
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    embed_windows_resources();
    bundle_catalog();
    bundle_remote_app();
}

/// Embeds `apps/remote/dist/**/*.gz` (made by `scripts/build-remote.sh`) so the node can serve its
/// web client from one binary. Without that folder the node still builds, and says on its home
/// page that the client is not part of this build.
fn bundle_remote_app() {
    let output =
        PathBuf::from(std::env::var_os("OUT_DIR").expect("OUT_DIR")).join("remote_assets.rs");
    let dist = Path::new("apps/remote/dist");
    let mut table = String::from("pub static REMOTE_APP: &[(&str, &[u8])] = &[\n");
    if std::env::var_os("CARGO_FEATURE_NODE").is_some() && dist.is_dir() {
        println!("cargo:rerun-if-changed=apps/remote/dist");
        let mut files = Vec::new();
        collect_files(dist, &mut files);
        files.sort();
        for file in files {
            let Some(name) = file
                .to_string_lossy()
                .strip_suffix(".gz")
                .map(str::to_owned)
            else {
                continue;
            };
            let url = name
                .replace('\\', "/")
                .strip_prefix("apps/remote/dist")
                .unwrap_or_default()
                .to_owned();
            let absolute = std::path::absolute(&file).expect("absolute asset path");
            let _ = writeln!(
                table,
                "    ({url:?}, include_bytes!({:?})),",
                absolute.to_string_lossy().replace('\\', "/")
            );
        }
    } else {
        // Watch the folder that would hold it, so building it later triggers a rebuild.
        println!("cargo:rerun-if-changed=apps/remote");
    }
    table.push_str("];\n");
    fs::write(output, table).expect("write the embedded web client table");
}

fn collect_files(dir: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_files(&path, files);
        } else {
            files.push(path);
        }
    }
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
    if std::env::var_os("CARGO_FEATURE_AGENT_ADAPTER_LAUNCHER").is_some() {
        binaries.push(("chxchx-agent-adapter", "ChxChx Agent Adapter"));
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
