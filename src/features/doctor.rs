use crate::features::mcp_diagnostics::{self, McpDiagnostic};
use crate::features::project_overview::{ProjectOverview, inspect_project};
use std::fs;
use std::path::{Path, PathBuf};

const TOOLS: [(&str, &str); 9] = [
    ("uv", "uv"),
    ("Git", "git"),
    ("Basic Memory", "basic-memory"),
    ("Serena", "serena"),
    ("Claude Code", "claude"),
    ("Codex", "codex"),
    ("OpenCode", "opencode"),
    ("Zellij", "zellij"),
    ("Sublime Text", "subl"),
];

#[derive(Clone, Debug)]
pub struct DoctorFinding {
    pub area: &'static str,
    pub status: &'static str,
    pub subject: String,
    pub detail: String,
}

#[derive(Clone, Debug)]
pub struct DoctorReport {
    pub project_name: String,
    pub project_root: String,
    pub profile: String,
    pub findings: Vec<DoctorFinding>,
    pub mcp: Vec<McpDiagnostic>,
}

pub fn diagnose_project(path: &Path) -> Result<DoctorReport, String> {
    let overview = inspect_project(path)?;
    let root = PathBuf::from(&overview.project.root);
    let mut findings = tool_findings();
    findings.extend(project_file_findings(&root));
    findings.extend(workspace_findings(&overview, &root));
    let mcp = mcp_diagnostics::diagnose_project_mcp(&root, &overview.project.name)?;
    Ok(DoctorReport {
        project_name: overview.project.name,
        project_root: overview.project.root,
        profile: overview.detection.profile,
        findings,
        mcp,
    })
}

impl DoctorReport {
    pub fn render(&self) -> String {
        let mut lines = vec![
            format!("Doctor · {}", self.project_name),
            format!("Ruta: {}", self.project_root),
            format!("Perfil: {}", self.profile),
        ];
        for area in ["HERRAMIENTAS", "ARCHIVOS DEL PROYECTO", "WORKSPACE"] {
            lines.push(String::new());
            lines.push(area.to_owned());
            lines.extend(
                self.findings
                    .iter()
                    .filter(|finding| finding.area == area)
                    .map(|finding| {
                        format!(
                            "{} {} · {}",
                            finding.status, finding.subject, finding.detail
                        )
                    }),
            );
        }
        lines.push(String::new());
        lines.push("INTEGRACIONES MCP (solo lectura)".to_owned());
        lines.extend(self.mcp.iter().map(render_mcp));
        lines.push(String::new());
        lines.push(
            "Diagnóstico estático: no ejecuta agentes, servidores ni comandos configurados."
                .to_owned(),
        );
        lines.join("\n")
    }
}

fn tool_findings() -> Vec<DoctorFinding> {
    TOOLS
        .iter()
        .map(|(name, command)| {
            let installed = crate::integrations::mcp_clients::find_executable(command).is_some();
            DoctorFinding {
                area: "HERRAMIENTAS",
                status: if installed { "OK" } else { "FALTA" },
                subject: (*name).to_owned(),
                detail: format!(
                    "{} `{command}`",
                    if installed {
                        "encontrado en PATH:"
                    } else {
                        "no encontrado en PATH:"
                    }
                ),
            }
        })
        .collect()
}

fn project_file_findings(root: &Path) -> Vec<DoctorFinding> {
    [
        ("AGENTS.md", "AGENTS.md"),
        ("CLAUDE.md", "CLAUDE.md"),
        (".ai", ".ai"),
        (".ai/chxchx-tech.toml", ".ai/chxchx-tech.toml"),
        (".ai/memory", ".ai/memory"),
        (
            ".ai/integrations/opencode-mcp.example.json",
            ".ai/integrations/opencode-mcp.example.json",
        ),
        ("docs/adr", "docs/adr"),
    ]
    .into_iter()
    .map(|(label, relative)| {
        let exists = root.join(relative).exists();
        DoctorFinding {
            area: "ARCHIVOS DEL PROYECTO",
            status: if exists { "OK" } else { "FALTA" },
            subject: label.to_owned(),
            detail: if exists {
                "existe".to_owned()
            } else {
                "no existe".to_owned()
            },
        }
    })
    .collect()
}

fn workspace_findings(overview: &ProjectOverview, root: &Path) -> Vec<DoctorFinding> {
    let workspace = &overview.workspace;
    let mut findings = Vec::new();
    if !workspace.config_exists {
        findings.push(finding(
            "FALTA",
            ".ai/chxchx-tech.toml",
            "ejecuta `chxchx-tech init` para preparar el workspace",
        ));
    } else if workspace.config_parseable != Some(true) || workspace.config_supported != Some(true) {
        findings.push(finding(
            "ERROR",
            ".ai/chxchx-tech.toml",
            workspace
                .config_validation_error
                .as_deref()
                .unwrap_or("no se pudo validar"),
        ));
    } else {
        findings.push(finding(
            "OK",
            ".ai/chxchx-tech.toml",
            &format!(
                "configuración v{} válida",
                workspace.config_version.unwrap_or_default()
            ),
        ));
        if workspace.config_version == Some(1) {
            findings.push(finding(
                "AVISO",
                "migración TOML",
                "la configuración v1 se actualizará en el próximo init completo",
            ));
        }
        findings.extend(configured_working_directories(root));
    }
    findings.push(finding(
        if workspace.trusted { "OK" } else { "AVISO" },
        "trust local",
        if workspace.trusted {
            "configuración confiable"
        } else {
            "proyecto no confiable; las acciones protegidas requieren aprobación"
        },
    ));
    findings
}

fn configured_working_directories(root: &Path) -> Vec<DoctorFinding> {
    let path = root.join(".ai").join("chxchx-tech.toml");
    let Ok(contents) = fs::read_to_string(path) else {
        return Vec::new();
    };
    let Ok(document) = toml::from_str::<toml::Value>(&contents) else {
        return Vec::new();
    };
    let Some(workspace) = document.get("workspace").and_then(toml::Value::as_table) else {
        return Vec::new();
    };
    ["processes", "agents"]
        .into_iter()
        .flat_map(|kind| {
            workspace
                .get(kind)
                .and_then(toml::Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(|entry| entry.as_table())
                .map(move |entry| {
                    let id = entry
                        .get("id")
                        .and_then(toml::Value::as_str)
                        .unwrap_or("sin-id");
                    let cwd = entry
                        .get("cwd")
                        .and_then(toml::Value::as_str)
                        .unwrap_or(".");
                    let directory = root.join(cwd);
                    let exists = directory.is_dir();
                    DoctorFinding {
                        area: "WORKSPACE",
                        status: if exists { "OK" } else { "ERROR" },
                        subject: format!("{id} cwd"),
                        detail: if exists {
                            format!("directorio disponible: {}", directory.display())
                        } else {
                            format!("no existe como directorio: {}", directory.display())
                        },
                    }
                })
        })
        .collect()
}

fn finding(status: &'static str, subject: &str, detail: &str) -> DoctorFinding {
    DoctorFinding {
        area: "WORKSPACE",
        status,
        subject: subject.to_owned(),
        detail: detail.to_owned(),
    }
}

fn render_mcp(item: &McpDiagnostic) -> String {
    format!(
        "{} {} / {} · {} · {}",
        item.status, item.client, item.server, item.scope, item.detail
    )
}
