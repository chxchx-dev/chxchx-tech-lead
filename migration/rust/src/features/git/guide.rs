//! What to do next, in plain words, from the state of the repository.
use super::types::GitStatus;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    Info,
    Warning,
    Done,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Situation {
    Conflicts,
    ReadyToCommit,
    Changes,
    Diverged,
    Behind,
    Ahead,
    Unpublished,
    NoCommits,
    Clean,
}

#[derive(Clone, Debug)]
pub struct Guidance {
    pub tone: Tone,
    pub situation: Situation,
    pub icon: &'static str,
    pub title: String,
    pub steps: Vec<String>,
}

pub struct GuideFacts {
    pub merge_in_progress: bool,
    pub has_remotes: bool,
    pub stash_count: usize,
}

pub fn guidance_for(status: &GitStatus, facts: &GuideFacts) -> Guidance {
    let conflicts = status.conflicts().len();
    let staged = status.staged_entries().len();
    let changes = status.changed_entries().len();
    let plural = |count: usize, one: &str, many: &str| {
        if count == 1 {
            one.to_owned()
        } else {
            many.replace("{}", &count.to_string())
        }
    };
    let (tone, situation, icon, title, steps): (Tone, Situation, &str, String, Vec<String>) =
        if conflicts > 0 {
            (
                Tone::Warning,
                Situation::Conflicts,
                "warning",
                plural(
                    conflicts,
                    "Hay 1 archivo en conflicto",
                    "Hay {} archivos en conflicto",
                ),
                vec![
                    "Abre cada archivo en conflicto y busca las marcas <<<<<<< ======= >>>>>>>."
                        .into(),
                    "Deja solo el código que quieras conservar y borra las marcas.".into(),
                    "Prepara el archivo con + y, cuando no quede ninguno, guarda los cambios."
                        .into(),
                    if facts.merge_in_progress {
                        "¿Prefieres empezar de nuevo? Usa «Cancelar la fusión»: no pierdes lo que ya habías guardado.".into()
                    } else {
                        "Si te equivocas, descarta el archivo con ↶ para volver a empezar.".into()
                    },
                ],
            )
        } else if staged > 0 {
            let mut steps = vec![
                "Escribe qué cambiaste en el mensaje del commit.".to_owned(),
                "Pulsa «Guardar (commit)».".to_owned(),
            ];
            if changes > 0 {
                steps.push(format!(
                    "Quedarán {changes} archivo(s) sin guardar: prepáralos con + si van en el mismo cambio."
                ));
            }
            (
                Tone::Info,
                Situation::ReadyToCommit,
                "git-commit",
                plural(
                    staged,
                    "1 archivo listo para guardar",
                    "{} archivos listos para guardar",
                ),
                steps,
            )
        } else if changes > 0 {
            (
                Tone::Info,
                Situation::Changes,
                "file-text",
                plural(changes, "Tienes 1 cambio sin guardar", "Tienes {} cambios sin guardar"),
                vec![
                    "Revisa los cambios con un clic en cada archivo.".into(),
                    "Prepara con + los que van juntos, o guarda todo de una vez.".into(),
                    "Si son cosas distintas, haz un commit por cada una: el historial queda más claro.".into(),
                ],
            )
        } else if status.behind > 0 && status.ahead > 0 {
            (
                Tone::Warning,
                Situation::Diverged,
                "branch",
                "Tu rama y el remoto se separaron".to_owned(),
                vec![
                    format!(
                        "Hay {} commit(s) nuevos en el remoto y {} tuyos sin enviar.",
                        status.behind, status.ahead
                    ),
                    "Pulsa «Recibir»: te propondrá fusionarlos (merge).".into(),
                    "Si hay conflictos, te guiaré para resolverlos. Después podrás enviar.".into(),
                ],
            )
        } else if status.behind > 0 {
            (
                Tone::Info,
                Situation::Behind,
                "arrow-down",
                format!("Hay {} novedad(es) en el remoto", status.behind),
                vec!["Pulsa «Recibir» para traerlas. Tu trabajo no se toca.".into()],
            )
        } else if status.ahead > 0 {
            (
                Tone::Info,
                Situation::Ahead,
                "arrow-up",
                format!("Tienes {} commit(s) por enviar", status.ahead),
                vec!["Pulsa «Enviar» para compartirlos con el equipo.".into()],
            )
        } else if !status.detached
            && status.upstream.is_none()
            && facts.has_remotes
            && !status.initial
        {
            (
                Tone::Info,
                Situation::Unpublished,
                "arrow-up",
                "Esta rama solo existe en tu equipo".to_owned(),
                vec!["Pulsa «Publicar rama» para subirla al remoto y poder compartirla.".into()],
            )
        } else if status.initial {
            (
                Tone::Info,
                Situation::NoCommits,
                "file",
                "Todavía no hay commits".to_owned(),
                vec![
                    "Crea o edita archivos: aparecerán aquí para guardarlos por primera vez."
                        .into(),
                ],
            )
        } else {
            let mut steps = vec!["No hay nada pendiente.".to_owned()];
            if facts.stash_count > 0 {
                steps.push(format!(
                    "Tienes {} apartado(s) (stash) por si quieres recuperar trabajo.",
                    facts.stash_count
                ));
            }
            (
                Tone::Done,
                Situation::Clean,
                "check",
                "Todo al día".to_owned(),
                steps,
            )
        };
    Guidance {
        tone,
        situation,
        icon,
        title,
        steps,
    }
}

#[cfg(test)]
mod tests {
    use super::super::parse::parse_status;
    use super::*;

    fn facts() -> GuideFacts {
        GuideFacts {
            merge_in_progress: false,
            has_remotes: true,
            stash_count: 2,
        }
    }

    fn guide(text: &str) -> Guidance {
        guidance_for(&parse_status(text), &facts())
    }

    #[test]
    fn guidance_follows_the_state_in_priority_order() {
        let head = "# branch.oid abc\0# branch.head main\0# branch.upstream origin/main\0";
        assert_eq!(
            guide(&format!(
                "{head}u UU N... 1 1 1 1 a b c x.rs\01 M. N... 1 1 1 a b y.rs\0"
            ))
            .situation,
            Situation::Conflicts
        );
        let ready = guide(&format!(
            "{head}1 M. N... 1 1 1 a b y.rs\01 .M N... 1 1 1 a b z.rs\0"
        ));
        assert_eq!(ready.situation, Situation::ReadyToCommit);
        assert!(ready.steps.iter().any(|step| step.contains("Quedarán 1")));
        assert_eq!(
            guide(&format!("{head}? n.md\0")).situation,
            Situation::Changes
        );
        assert_eq!(
            guide(&format!("{head}# branch.ab +1 -2\0")).situation,
            Situation::Diverged
        );
        assert_eq!(
            guide(&format!("{head}# branch.ab +0 -3\0")).situation,
            Situation::Behind
        );
        assert_eq!(
            guide(&format!("{head}# branch.ab +4 -0\0")).situation,
            Situation::Ahead
        );
        assert_eq!(
            guide("# branch.oid abc\0# branch.head feature\0").situation,
            Situation::Unpublished
        );
        assert_eq!(
            guide("# branch.oid (initial)\0# branch.head main\0").situation,
            Situation::NoCommits
        );
        let clean = guide(&format!("{head}# branch.ab +0 -0\0"));
        assert_eq!(clean.situation, Situation::Clean);
        assert!(clean.steps.last().unwrap().contains("2 apartado"));
    }
}
