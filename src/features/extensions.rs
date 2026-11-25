//! Official extensions (ADR 0018): a closed list compiled into the product.
//!
//! There is no code loading and no third-party extensions. An extension is off until the person
//! turns it on in Settings, after being told what it creates, where, and how to remove it. While
//! off it opens no file, starts no thread and shows no icon: only a boolean is read.

/// Id of Colmena, the board and decisions extension.
pub const HIVE: &str = "hive";

/// What the person is told before turning an extension on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Extension {
    pub id: &'static str,
    pub name: &'static str,
    pub tagline: &'static str,
    /// What turning it on creates and where.
    pub creates: &'static str,
    /// What it costs while on.
    pub cost: &'static str,
    /// How to get rid of what it created.
    pub removal: &'static str,
    /// The icon in the rail and in Settings.
    pub icon: &'static str,
}

const ALL: &[Extension] = &[Extension {
    id: HIVE,
    name: "Colmena",
    tagline: "Un tablero vivo para lo que planean tus agentes: tareas, roadmap y decisiones que esperan por ti.",
    creates: "Una base de datos SQLite por proyecto en la carpeta de datos de Studio (carpeta «hive»). \
              No escribe nada dentro de tus repositorios salvo que actives la exportación del tablero.",
    cost: "Sin actividad en reposo: unos pocos MB de memoria y nada de CPU. Indexar un proyecto grande \
           usa un hilo de baja prioridad y se puede cancelar.",
    removal: "Desactívala para ocultarla y conservar los datos, o bórralos desde aquí: antes se guarda \
              una copia exportada.",
    icon: "hive",
}];

/// The extensions this build offers (those whose feature was compiled in).
pub fn all() -> impl Iterator<Item = &'static Extension> {
    ALL.iter().filter(|extension| available(extension.id))
}

pub fn find(id: &str) -> Option<&'static Extension> {
    all().find(|extension| extension.id == id)
}

/// Whether this build contains the extension at all (a build without GUI or without the feature
/// does not).
pub fn available(id: &str) -> bool {
    id == HIVE && cfg!(feature = "hive")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_list_is_closed_and_ids_are_unique() {
        let ids: Vec<_> = ALL.iter().map(|extension| extension.id).collect();
        let mut sorted = ids.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(ids.len(), sorted.len());
        assert!(find("something-else").is_none());
    }

    #[test]
    fn every_extension_says_what_it_creates_costs_and_how_to_remove_it() {
        for extension in ALL {
            assert!(!extension.creates.is_empty());
            assert!(!extension.cost.is_empty());
            assert!(!extension.removal.is_empty());
        }
    }

    #[cfg(feature = "hive")]
    #[test]
    fn hive_is_offered_when_it_is_compiled_in() {
        assert_eq!(find(HIVE).map(|extension| extension.name), Some("Colmena"));
    }
}
