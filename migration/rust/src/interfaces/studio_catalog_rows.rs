//! List rows for each catalog tab.
use super::ui::{action, bulk, chip, item, matches};
use super::*;
use crate::features::catalog_view::CatalogView;
use crate::features::project_mods::ModsChange;
use crate::features::project_skills::SkillsChange;
use iced::widget::column;

type Rows<'a> = iced::widget::Column<'a, Message>;

pub(super) fn skills<'a>(data: &'a CatalogView, filter: &str) -> Rows<'a> {
    let mut rows = column![].spacing(8);
    let recommended: Vec<String> = data
        .skills
        .iter()
        .filter(|skill| skill.recommended && !skill.enabled)
        .map(|skill| skill.name.clone())
        .collect();
    if !recommended.is_empty() {
        rows = rows.push(bulk(
            format!("{} skills recomendadas sin habilitar", recommended.len()),
            "Habilitar recomendadas",
            Message::Catalog(Event::Skills(SkillsChange::Enable(recommended))),
        ));
    }
    for skill in data.skills.iter().filter(|skill| {
        matches(
            filter,
            &[&skill.name, &skill.description, &skill.tags.join(" ")],
        )
    }) {
        let mut chips = Vec::new();
        if skill.enabled {
            chips.push(chip("HABILITADA", design::SUCCESS));
        }
        if skill.recommended {
            chips.push(chip("RECOMENDADA", design::BRAND_CYAN));
        }
        let change = if skill.enabled {
            SkillsChange::Disable(vec![skill.name.clone()])
        } else {
            SkillsChange::Enable(vec![skill.name.clone()])
        };
        let (label, style): (&str, fn(&iced::Theme, button::Status) -> button::Style) =
            if skill.enabled {
                ("Deshabilitar", styles::secondary)
            } else {
                ("Habilitar", styles::primary)
            };
        rows = rows.push(item(
            &skill.name,
            &skill.description,
            chips,
            skill.reasons.clone(),
            action(label, style, Message::Catalog(Event::Skills(change))),
        ));
    }
    rows
}

pub(super) fn packs<'a>(data: &'a CatalogView, filter: &str) -> Rows<'a> {
    let mut rows = column![].spacing(8);
    for pack in data
        .packs
        .iter()
        .filter(|pack| matches(filter, &[&pack.name, &pack.description]))
    {
        let mut chips = vec![chip(
            format!("{}/{} SKILLS", pack.enabled_skills, pack.skills.len()),
            design::BRAND_BLUE,
        )];
        if pack.recommended {
            chips.push(chip("DETECTADO", design::BRAND_CYAN));
        }
        let complete = pack.enabled_skills == pack.skills.len();
        let mut notes = vec![format!("Incluye: {}", pack.skills.join(", "))];
        notes.extend(pack.reasons.iter().cloned());
        let apply = if complete {
            chip("COMPLETO", design::SUCCESS)
        } else {
            action(
                "Aplicar pack",
                styles::primary,
                Message::Catalog(Event::Skills(SkillsChange::Pack(pack.name.clone()))),
            )
        };
        rows = rows.push(item(&pack.name, &pack.description, chips, notes, apply));
    }
    rows
}

pub(super) fn mods<'a>(data: &'a CatalogView, filter: &str) -> Rows<'a> {
    let mut rows = column![].spacing(8);
    if data
        .mods
        .iter()
        .any(|module| module.recommended && !module.applied && !module.conflict)
    {
        rows = rows.push(bulk(
            "Hay Mods recomendados para este proyecto".to_owned(),
            "Instalar recomendados",
            Message::Catalog(Event::Mods(ModsChange::Apply(None))),
        ));
    }
    for module in data
        .mods
        .iter()
        .filter(|module| matches(filter, &[&module.name, &module.description]))
    {
        let mut chips = Vec::new();
        if module.applied {
            chips.push(chip("INSTALADO", design::SUCCESS));
        }
        if module.recommended {
            chips.push(chip("RECOMENDADO", design::BRAND_CYAN));
        }
        if module.conflict {
            chips.push(chip("ARCHIVO AJENO", design::WARNING));
        }
        let control = if module.conflict {
            chip("NO SE SOBRESCRIBE", design::FAINT)
        } else if module.applied {
            action(
                "Retirar",
                styles::secondary,
                Message::Catalog(Event::Mods(ModsChange::Remove(vec![module.name.clone()]))),
            )
        } else {
            action(
                "Instalar",
                styles::primary,
                Message::Catalog(Event::Mods(ModsChange::Apply(Some(vec![
                    module.name.clone(),
                ])))),
            )
        };
        rows = rows.push(item(
            &module.name,
            &module.description,
            chips,
            module.reasons.clone(),
            control,
        ));
    }
    rows
}
