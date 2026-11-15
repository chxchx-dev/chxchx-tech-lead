//! The "Modelos locales" tab: your machine, Ollama's state and the models that fit.
use super::super::*;
use super::{Event, State};
use crate::integrations::local_models::{Candidate, Snapshot, size_label};
use iced::widget::{column, progress_bar, tooltip};

fn msg(event: Event) -> Message {
    Message::Local(event)
}

fn stat<'a>(label: &'static str, value: String) -> Element<'a, Message> {
    column![
        text(label).size(10).color(design::FAINT),
        text(value).size(14).font(design::UI_BOLD),
    ]
    .spacing(2)
    .width(Fill)
    .into()
}

fn card<'a>(body: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    container(body)
        .padding(16)
        .width(Fill)
        .style(styles::card)
        .into()
}

fn chip<'a>(label: String, tint: iced::Color) -> Element<'a, Message> {
    container(text(label).size(10).font(design::UI_BOLD))
        .padding([2, 8])
        .style(styles::chip(tint))
        .into()
}

fn fit_chip<'a>(fit: &str) -> Element<'a, Message> {
    match fit {
        "gpu" => chip("GPU · rápido".into(), design::SUCCESS),
        "partial" => chip("GPU + RAM".into(), design::ACCENT),
        "cpu" => chip("Solo CPU".into(), design::WARNING),
        _ => chip("No cabe".into(), design::DANGER),
    }
}

fn hardware(snapshot: &Snapshot) -> Element<'_, Message> {
    let hw = &snapshot.hardware;
    card(
        column![
            text("Tu equipo").size(13).font(design::UI_BOLD),
            row![
                stat("MEMORIA", size_label(hw.total_ram)),
                stat("LIBRE AHORA", size_label(hw.available_ram)),
                stat("NÚCLEOS", hw.cpu_cores.to_string()),
                stat("DISCO LIBRE", size_label(hw.disk_free)),
            ]
            .spacing(12),
            row![
                icons::icon("cpu", design::MUTED, 13.0),
                text(format!("Aceleración: {}", hw.accelerator))
                    .size(11)
                    .color(design::MUTED),
            ]
            .spacing(6)
            .align_y(iced::Center),
            text("Un modelo corre rápido solo si cabe en la memoria de la GPU; si no, se reparte con la RAM y va más lento.")
                .size(11)
                .color(design::FAINT),
        ]
        .spacing(10),
    )
}

fn ollama(snapshot: &Snapshot) -> Element<'_, Message> {
    let o = &snapshot.ollama;
    let (tint, headline) = if !o.installed {
        (design::WARNING, "Ollama no está instalado".to_owned())
    } else if !o.running {
        (
            design::WARNING,
            "Ollama está instalado pero no responde".to_owned(),
        )
    } else {
        (
            design::SUCCESS,
            format!("Ollama {} activo · {} modelos", o.version, o.models.len()),
        )
    };
    let mut body = column![
        row![
            icons::icon("dot", tint, 10.0),
            text(headline).size(13).font(design::UI_BOLD),
        ]
        .spacing(6)
        .align_y(iced::Center)
    ]
    .spacing(8);
    if !o.installed {
        body = body.push(
            text("Ollama ejecuta modelos en tu equipo, gratis y sin enviar nada a internet. Instálalo con el método oficial y vuelve a diagnosticar:")
                .size(11)
                .color(design::MUTED),
        );
        if let Some(command) = &o.install_command {
            body = body.push(copyable(command.clone()));
        }
        body = body.push(copyable(o.install_url.clone()));
    } else if !o.running {
        body = body.push(
            text(format!(
                "Abre la aplicación de Ollama o ejecuta `ollama serve` (escucha en {}).",
                o.base_url
            ))
            .size(11)
            .color(design::MUTED),
        );
    }
    if !snapshot.opencode_installed {
        body = body.push(
            text(format!(
                "OpenCode no está instalado: {}. Lo necesitas para usar estos modelos como agente.",
                snapshot.opencode_hint
            ))
            .size(11)
            .color(design::WARNING),
        );
    }
    card(body)
}

fn copyable<'a>(value: String) -> Element<'a, Message> {
    row![
        container(
            text(value.clone())
                .size(11)
                .font(design::MONO)
                .wrapping(iced::widget::text::Wrapping::None)
        )
        .padding([5, 10])
        .width(Fill)
        .clip(true)
        .style(styles::chip_surface(false)),
        styles::tip(
            button(icons::icon("file-text", design::MUTED, 13.0))
                .padding(6)
                .style(styles::ghost(false))
                .on_press(msg(Event::Copy(value))),
            "Copiar",
            tooltip::Position::Left,
        ),
    ]
    .spacing(6)
    .align_y(iced::Center)
    .into()
}

fn model_row<'a>(local: &State, snapshot: &Snapshot, model: &'a Candidate) -> Element<'a, Message> {
    let ready = snapshot.ollama.running && snapshot.opencode_installed && !local.busy();
    let fits = model.fit != "none";
    let mut title = row![
        text(&model.label).size(13).font(design::UI_BOLD),
        fit_chip(&model.fit),
    ]
    .spacing(8)
    .align_y(iced::Center);
    if model.recommended {
        title = title.push(chip("★ Recomendado".into(), design::BRAND_CYAN));
    }
    if model.configured {
        title = title.push(chip("En OpenCode".into(), design::SUCCESS));
    } else if model.downloaded {
        title = title.push(chip("Descargado".into(), design::MUTED));
    }
    let mut actions = row![].spacing(6).align_y(iced::Center);
    let connected = super::connected_name(snapshot, &model.tag);
    if model.downloaded || model.configured {
        actions = actions.push(
            button(text("Quitar").size(11))
                .padding([5, 10])
                .style(styles::secondary)
                .on_press_maybe(
                    (snapshot.ollama.running && !local.busy())
                        .then(|| msg(Event::Remove(model.tag.clone()))),
                ),
        );
    }
    match connected {
        // Connected: use it like any other agent, in the visual chat or a terminal.
        Some(name) => {
            actions = actions
                .push(
                    button(
                        row![
                            icons::icon("terminal", design::TEXT, 12.0),
                            text("Terminal").size(11)
                        ]
                        .spacing(6)
                        .align_y(iced::Center),
                    )
                    .padding([5, 10])
                    .style(styles::secondary)
                    .on_press(msg(Event::Terminal(name.clone()))),
                )
                .push(
                    button(
                        row![
                            icons::icon("sparkle", design::TEXT, 12.0),
                            text("Chat").size(11)
                        ]
                        .spacing(6)
                        .align_y(iced::Center),
                    )
                    .padding([5, 12])
                    .style(styles::primary)
                    .on_press(msg(Event::Chat(name))),
                );
        }
        None => {
            let label = if model.downloaded {
                "Conectar a OpenCode"
            } else {
                "Instalar y conectar"
            };
            actions = actions.push(
                button(text(label).size(11))
                    .padding([5, 12])
                    .style(styles::primary)
                    .on_press_maybe(
                        (ready && fits && model.disk_ok)
                            .then(|| msg(Event::Install(model.tag.clone()))),
                    ),
            );
        }
    }
    container(
        row![
            column![
                title,
                text(format!(
                    "{} · contexto {}K · necesita ~{} de memoria",
                    size_label(model.size),
                    model.context_k,
                    size_label(model.required)
                ))
                .size(11)
                .color(design::MUTED),
                text(&model.note).size(11).color(design::FAINT),
                text(&model.summary).size(11).color(if fits {
                    design::MUTED
                } else {
                    design::DANGER
                }),
            ]
            .spacing(3)
            .width(Fill),
            actions,
        ]
        .spacing(12)
        .align_y(iced::Center),
    )
    .padding([10, 12])
    .width(Fill)
    .style(styles::chip_surface(model.recommended))
    .into()
}

fn working(local: &State) -> Option<Element<'_, Message>> {
    local.job.as_ref()?;
    let p = &local.progress;
    let fraction = if p.total > 0 {
        (p.done as f32 / p.total as f32).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let detail = if p.total > 0 {
        format!(
            "{} de {} ({:.0} %)",
            size_label(p.done),
            size_label(p.total),
            fraction * 100.0
        )
    } else {
        String::new()
    };
    Some(card(
        column![
            row![
                text(&p.text).size(12).font(design::UI_BOLD).width(Fill),
                button(text("Cancelar").size(11))
                    .padding([4, 10])
                    .style(styles::secondary)
                    .on_press(msg(Event::Cancel)),
            ]
            .align_y(iced::Center),
            progress_bar(0.0..=1.0, fraction).girth(6),
            text(detail).size(11).color(design::MUTED),
        ]
        .spacing(8),
    ))
}

pub(in crate::interfaces::studio) fn view(state: &Dashboard) -> Element<'_, Message> {
    let local = &state.local;
    let mut page = column![
        row![
            column![
                text("Modelos locales").size(20).font(design::UI_BOLD),
                text("Ejecuta modelos en tu equipo con Ollama y úsalos desde OpenCode: privados, sin conexión y sin costo por uso.")
                    .size(12)
                    .color(design::MUTED),
            ]
            .spacing(4)
            .width(Fill),
            styles::tip(
                button(icons::icon("refresh", design::MUTED, 14.0))
                    .padding(7)
                    .style(styles::ghost(false))
                    .on_press_maybe((!local.loading && !local.busy()).then(|| msg(Event::Refresh))),
                "Volver a diagnosticar el equipo",
                tooltip::Position::Left,
            ),
        ]
        .align_y(iced::Center),
    ]
    .spacing(14)
    .padding([20, 28])
    .max_width(900);
    if let Some(error) = &local.error {
        page = page.push(
            container(text(error).size(12))
                .padding([8, 12])
                .width(Fill)
                .style(styles::tinted(design::WARNING)),
        );
    }
    if let Some(done) = &local.result {
        page = page.push(
            container(text(done).size(12))
                .padding([8, 12])
                .width(Fill)
                .style(styles::tinted(design::SUCCESS)),
        );
    }
    if let Some(job) = working(local) {
        page = page.push(job);
    }
    match &local.snapshot {
        None => {
            page = page.push(
                text(if local.loading {
                    "Diagnosticando tu equipo…"
                } else {
                    "Sin diagnóstico todavía."
                })
                .size(12)
                .color(design::MUTED),
            );
        }
        Some(snapshot) => {
            page = page.push(hardware(snapshot)).push(ollama(snapshot));
            page = page.push(
                row![
                    text("Recomendados para este equipo")
                        .size(13)
                        .font(design::UI_BOLD)
                        .width(Fill),
                    iced::widget::checkbox(local.set_default)
                        .label("Usar como modelo predeterminado de OpenCode")
                        .size(14)
                        .text_size(11)
                        .on_toggle(|_| msg(Event::ToggleDefault)),
                ]
                .align_y(iced::Center),
            );
            for model in &snapshot.models {
                page = page.push(model_row(local, snapshot, model));
            }
            page = page.push(
                text("Los tamaños y requisitos son estimaciones. Instalar descarga el modelo con `ollama pull`, crea una variante con más contexto y la conecta al opencode.json de este proyecto (con respaldo).")
                    .size(11)
                    .color(design::FAINT),
            );
        }
    }
    container(styles::vscroll(container(page).center_x(Fill)).height(Fill))
        .width(Fill)
        .height(Fill)
        .style(styles::surface(design::BG_BASE))
        .into()
}
