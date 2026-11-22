//! Where the client is, kept in the address (`#/chat/proyecto/claude`) so the back button works
//! and a page can be reopened where it was. Also the pairing code a QR puts in the address.

use chxchx_protocol::{JobId, ProjectId, Provider};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Route {
    Home,
    Projects,
    Chat {
        project: ProjectId,
        provider: Provider,
    },
    Jobs,
    Job(JobId),
    Devices,
    Settings,
}

/// The tabs of the bottom bar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tab {
    Home,
    Projects,
    Jobs,
    Devices,
    Settings,
}

impl Route {
    pub fn tab(&self) -> Tab {
        match self {
            Route::Home => Tab::Home,
            Route::Projects | Route::Chat { .. } => Tab::Projects,
            Route::Jobs | Route::Job(_) => Tab::Jobs,
            Route::Devices => Tab::Devices,
            Route::Settings => Tab::Settings,
        }
    }

    pub fn to_hash(&self) -> String {
        match self {
            Route::Home => "#/".to_owned(),
            Route::Projects => "#/projects".to_owned(),
            Route::Chat { project, provider } => {
                format!("#/chat/{project}/{}", provider.as_str())
            }
            Route::Jobs => "#/jobs".to_owned(),
            Route::Job(id) => format!("#/job/{id}"),
            Route::Devices => "#/devices".to_owned(),
            Route::Settings => "#/settings".to_owned(),
        }
    }

    /// Anything unknown or malformed is the home page.
    pub fn from_hash(hash: &str) -> Route {
        let path = hash.trim_start_matches('#').trim_start_matches('/');
        let mut parts = path.split('/');
        match (
            parts.next().unwrap_or_default(),
            parts.next(),
            parts.next(),
            parts.next(),
        ) {
            ("projects", None, ..) => Route::Projects,
            ("chat", Some(project), Some(provider), None) => {
                let provider = [Provider::Claude, Provider::Codex, Provider::OpenCode]
                    .into_iter()
                    .find(|candidate| candidate.as_str() == provider);
                match (ProjectId::parse(project), provider) {
                    (Ok(project), Some(provider)) => Route::Chat { project, provider },
                    _ => Route::Home,
                }
            }
            ("jobs", None, ..) => Route::Jobs,
            ("job", Some(id), None, None) => JobId::parse(id).map_or(Route::Home, Route::Job),
            ("devices", None, ..) => Route::Devices,
            ("settings", None, ..) => Route::Settings,
            _ => Route::Home,
        }
    }
}

/// The pairing code in `#code=ABCDE-FGH23` (a QR puts it there), if it looks like one.
pub fn code_in_hash(hash: &str) -> Option<String> {
    let query = hash.trim_start_matches('#');
    let code = query
        .split('&')
        .find_map(|pair| pair.strip_prefix("code="))?
        .to_ascii_uppercase();
    let letters = code.replace('-', "");
    (letters.len() == 10 && letters.chars().all(|c| c.is_ascii_alphanumeric()))
        .then(|| format!("{}-{}", &letters[..5], &letters[5..]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_route_survives_a_round_trip_through_the_address() {
        let routes = [
            Route::Home,
            Route::Projects,
            Route::Chat {
                project: ProjectId::parse("mi.proyecto-1").unwrap(),
                provider: Provider::OpenCode,
            },
            Route::Jobs,
            Route::Job(JobId::parse("01a1-9db8f-1").unwrap()),
            Route::Devices,
            Route::Settings,
        ];
        for route in routes {
            assert_eq!(
                Route::from_hash(&route.to_hash()),
                route,
                "{}",
                route.to_hash()
            );
        }
    }

    #[test]
    fn an_unknown_or_hostile_address_is_the_home_page() {
        for hash in [
            "",
            "#",
            "#/",
            "#/nope",
            "#/chat",
            "#/chat/a",
            "#/chat/a/robot",
            "#/chat/../etc/claude",
            "#/chat/a b/claude",
            "#/chat/a/claude/extra",
            "#/job",
            "#/job/a/b",
            "#/job/../x",
            "#/projects/extra",
        ] {
            assert_eq!(Route::from_hash(hash), Route::Home, "{hash:?}");
        }
    }

    #[test]
    fn a_chat_belongs_to_the_projects_tab_and_a_job_to_the_jobs_tab() {
        let chat = Route::Chat {
            project: ProjectId::parse("p").unwrap(),
            provider: Provider::Claude,
        };
        assert_eq!(chat.tab(), Tab::Projects);
        assert_eq!(Route::Job(JobId::parse("j").unwrap()).tab(), Tab::Jobs);
    }

    #[test]
    fn a_pairing_code_in_the_address_is_normalised_and_checked() {
        assert_eq!(
            code_in_hash("#code=abcde-fgh23").as_deref(),
            Some("ABCDE-FGH23")
        );
        assert_eq!(
            code_in_hash("#x=1&code=ABCDEFGH23").as_deref(),
            Some("ABCDE-FGH23")
        );
        for bad in [
            "",
            "#code=",
            "#code=ABC",
            "#code=ABCDE-FGH2!",
            "#code=ABCDE-FGH234",
            "#/projects",
        ] {
            assert_eq!(code_in_hash(bad), None, "{bad:?}");
        }
    }
}
