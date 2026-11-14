//! What went wrong with a git command, in terms the interface can act on.
use crate::integrations::git::Output;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Problem {
    /// Git does not know the user's name and email.
    Identity,
    /// The remote needs a login Studio does not ask for.
    Credentials,
    /// The remote has commits we do not have.
    Rejected,
    /// `pull --ff-only` cannot fast-forward.
    Diverged,
    /// The branch is not tracking a remote branch.
    NoUpstream,
    /// The repository has no remote at all.
    NoRemote,
    Other,
}

#[derive(Clone, Debug)]
pub struct GitFailure {
    pub problem: Problem,
    pub message: String,
}

impl GitFailure {
    pub fn other(message: impl Into<String>) -> Self {
        Self {
            problem: Problem::Other,
            message: message.into(),
        }
    }
}

impl std::fmt::Display for GitFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

fn mentions(text: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| text.contains(needle))
}

pub fn classify(stderr: &str) -> Problem {
    if mentions(
        stderr,
        &[
            "Please tell me who you are",
            "unable to auto-detect email",
            "empty ident name",
        ],
    ) {
        Problem::Identity
    } else if mentions(
        stderr,
        &[
            "Authentication failed",
            "could not read Username",
            "could not read Password",
            "terminal prompts disabled",
            "Permission denied (publickey",
            "Host key verification failed",
            "Invalid username or password",
        ],
    ) {
        Problem::Credentials
    } else if mentions(
        stderr,
        &[
            "no upstream branch",
            "has no upstream",
            "There is no tracking information",
        ],
    ) {
        Problem::NoUpstream
    } else if mentions(
        stderr,
        &[
            "No configured push destination",
            "No remote repository specified",
            "does not appear to be a git repository",
        ],
    ) && !stderr.contains("Permission denied")
    {
        Problem::NoRemote
    } else if mentions(
        stderr,
        &[
            "non-fast-forward",
            "fetch first",
            "[rejected]",
            "Updates were rejected",
        ],
    ) {
        Problem::Rejected
    } else if mentions(
        stderr,
        &[
            "Not possible to fast-forward",
            "divergent branches",
            "need to specify how to reconcile",
        ],
    ) {
        Problem::Diverged
    } else {
        Problem::Other
    }
}

pub fn failure(output: &Output) -> GitFailure {
    if output.truncated {
        return GitFailure::other("La salida de Git es demasiado grande y se recortó.");
    }
    let problem = classify(&output.stderr);
    let message = match problem {
        Problem::Identity => "Git no sabe quién eres. Configura tu nombre y correo: git config --global user.name \"Tu nombre\" y git config --global user.email \"tu@correo\".".to_owned(),
        Problem::Credentials => "El remoto pide credenciales que Studio no solicita. Inicia sesión desde tu configuración de Git (gestor de credenciales o clave SSH) y vuelve a intentarlo.".to_owned(),
        Problem::Rejected => "El remoto tiene commits que no tienes. Pulsa «Recibir» primero y vuelve a enviar.".to_owned(),
        Problem::Diverged => "Tu rama y el remoto se separaron: usa «Recibir y fusionar».".to_owned(),
        Problem::NoUpstream => "Esta rama aún no sigue una rama remota. Usa «Publicar rama».".to_owned(),
        Problem::NoRemote => "Este repositorio no tiene un remoto configurado.".to_owned(),
        Problem::Other => {
            let detail = output
                .stderr
                .lines()
                .map(str::trim)
                .find(|line| !line.is_empty())
                .map(|line| line.chars().take(300).collect::<String>());
            detail.unwrap_or_else(|| match output.code {
                Some(code) => format!("Git terminó con el código {code}."),
                None => "Git se detuvo antes de terminar.".to_owned(),
            })
        }
    };
    GitFailure { problem, message }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failures_are_classified_and_explained() {
        assert_eq!(
            classify("fatal: unable to auto-detect email address"),
            Problem::Identity
        );
        assert_eq!(
            classify("fatal: could not read Username for 'https://x'"),
            Problem::Credentials
        );
        assert_eq!(
            classify("! [rejected] main -> main (fetch first)"),
            Problem::Rejected
        );
        assert_eq!(
            classify("fatal: Not possible to fast-forward, aborting."),
            Problem::Diverged
        );
        assert_eq!(
            classify("fatal: The current branch x has no upstream branch."),
            Problem::NoUpstream
        );
        assert_eq!(
            classify("fatal: No configured push destination."),
            Problem::NoRemote
        );
        let output = Output {
            code: Some(1),
            stderr: "\nerror: pathspec 'x' did not match\n".into(),
            ..Default::default()
        };
        assert_eq!(
            failure(&output).message,
            "error: pathspec 'x' did not match"
        );
    }
}
