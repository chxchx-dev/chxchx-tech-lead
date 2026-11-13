//! Build-time packaging: the Windows icon and the bundled skills/packs/mods catalog.
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    embed_windows_icon();
    bundle_catalog();
}

fn embed_windows_icon() {
    println!("cargo:rerun-if-changed=assets/brand/studio.rc");
    println!("cargo:rerun-if-changed=assets/brand/chxchx.ico");
    let windows = std::env::var("CARGO_CFG_TARGET_OS").is_ok_and(|os| os == "windows");
    let studio = std::env::var_os("CARGO_FEATURE_STUDIO").is_some();
    if windows && studio {
        // The icon is cosmetic: a missing resource compiler must not break the build.
        if let Err(error) = embed_resource::compile_for(
            "assets/brand/studio.rc",
            ["chxchx-studio"],
            embed_resource::NONE,
        )
        .manifest_optional()
        {
            println!("cargo:warning=No se pudo incrustar el icono de Studio: {error}");
        }
    }
}

/// Embeds the catalog that the legacy product ships, so both products read one source.
/// While the migration lasts it lives under `src/chxchx_tech_lead`; when Rust is
/// promoted, move the directory and update `LEGACY`.
const LEGACY: &str = "../../src/chxchx_tech_lead";

fn bundle_catalog() {
    let base = Path::new(LEGACY);
    let skills = base.join("skills/library/skills");
    let packs = base.join("skills/library/packs");
    let mods = base.join("core/mod_catalog");
    for path in [&skills, &packs, &mods] {
        println!("cargo:rerun-if-changed={}", path.display());
    }
    let mut out = String::new();
    write_entries(&mut out, "SKILLS", &skills, "skill.toml", true);
    write_entries(&mut out, "MODS", &mods, "mod.toml", true);
    write_entries(&mut out, "PACKS", &packs, "", false);
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
