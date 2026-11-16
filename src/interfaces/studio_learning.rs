//! A short first-run path on Home, focused on preparing and safely starting a project.
use super::pages::eyebrow;
use super::*;
use iced::widget::{column, progress_bar};

struct Step {
    title: &'static str,
    detail: &'static str,
    label: &'static str,
    done: bool,
    action: Option<Message>,
}

fn steps(state: &Dashboard) -> Vec<Step> {
    let workspace = state
        .overview
        .as_ref()
        .ok()
        .map(|project| &project.workspace);
    let configured = workspace.is_some_and(|workspace| {
        workspace.config_exists && workspace.config_validation_error.is_none()
    });
    let trusted = workspace.is_some_and(|workspace| workspace.trusted);
    let first_chat = state
        .agents
        .catalog
        .agents
        .iter()
        .find(|agent| agent.provider.has_visual_chat() && agent.available())
        .map(|agent| Message::Agents(agents::Event::Chat(agent.id.clone())));
    let started = !state.chat.chats.is_empty() || state.terminal.has_sessions();
    let (start_detail, start_label, start_action) = if !trusted {
        (
            "Primero concede confianza local para permitir que Studio abra agentes y terminales aquí.",
            "Revisar confianza",
            Message::PreviewTrust { desired: true },
        )
    } else if let Some(chat) = first_chat {
        (
            "Abre el chat de un agente detectado. Puedes revisar sus permisos antes de cada acción.",
            "Abrir chat",
            chat,
        )
    } else {
        (
            "Abre Agentes para consultar qué asistentes están disponibles y cómo conectarlos.",
            "Ver agentes",
            Message::Shell(shell::Event::ShowSide(shell::Side::Agents)),
        )
    };

    vec![
        Step {
            title: "Prepara el proyecto",
            detail: "Crea la configuración y las guías; revisa los cambios antes de aplicarlos.",
            label: "Revisar preparación",
            done: configured,
            action: Some(Message::PreviewInit { minimal: false }),
        },
        Step {
            title: "Decide la confianza local",
            detail: "Permite abrir agentes y terminales en este proyecto. Guardarla no ejecuta nada y puedes revocarla.",
            label: "Revisar confianza",
            done: trusted,
            action: Some(Message::PreviewTrust { desired: true }),
        },
        Step {
            title: "Empieza a trabajar",
            detail: start_detail,
            label: start_label,
            done: started,
            action: Some(start_action),
        },
    ]
}

pub(super) fn card(state: &Dashboard) -> Element<'_, Message> {
    let steps = steps(state);
    let done = steps.iter().filter(|step| step.done).count();
    let next = steps.iter().position(|step| !step.done);
    let mut list = column![].spacing(8);
    for (index, step) in steps.into_iter().enumerate() {
        list = list.push(row_of(step, Some(index) == next));
    }
    container(
        column![
            row![
                column![
                    eyebrow("PRIMEROS PASOS"),
                    text("Deja listo tu espacio").size(16).font(design::UI_BOLD),
                    text("La preparación y la confianza se revisan antes de guardar.")
                        .size(12)
                        .color(design::MUTED),
                ]
                .spacing(4),
                Space::new().width(Fill),
                text(format!("{done} de 3"))
                    .size(12)
                    .font(design::UI_BOLD)
                    .color(design::ACCENT),
            ]
            .align_y(iced::Center),
            progress_bar(0.0..=3.0, done as f32)
                .girth(5)
                .style(|_: &iced::Theme| iced::widget::progress_bar::Style {
                    background: iced::Background::Color(design::BORDER),
                    bar: iced::Background::Color(design::ACCENT),
                    border: iced::Border::default().rounded(3.0),
                }),
            list,
        ]
        .spacing(12),
    )
    .padding(20)
    .width(Fill)
    .style(styles::card)
    .into()
}

fn row_of(step: Step, next: bool) -> Element<'static, Message> {
    let (icon, tint) = if step.done {
        ("check", design::SUCCESS)
    } else if next {
        ("arrow-right", design::ACCENT)
    } else {
        ("dot", design::FAINT)
    };
    let action = button(text(step.label).size(12))
        .padding([5, 12])
        .style(if next {
            styles::primary
        } else {
            styles::secondary
        })
        .on_press_maybe(step.action);
    container(
        row![
            icons::icon(icon, tint, 15.0),
            column![
                text(step.title).size(13).font(design::UI_BOLD),
                text(step.detail).size(11).color(design::MUTED),
            ]
            .spacing(2)
            .width(Fill),
            action,
        ]
        .spacing(12)
        .align_y(iced::Center),
    )
    .padding([9, 12])
    .width(Fill)
    .style(move |theme: &iced::Theme| {
        if next {
            styles::tinted(design::ACCENT)(theme)
        } else {
            styles::well(theme)
        }
    })
    .into()
}
