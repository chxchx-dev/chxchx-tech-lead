//! The TUI end to end without a terminal: real pages drawn into an in-memory screen, real keys
//! and clicks, read back as the text a person would see.
use super::app::{CatalogTab, Done};
use super::*;
use ratatui::backend::TestBackend;
use ratatui::crossterm::event::{KeyEvent, MouseEvent};

fn project(name: &str) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!(
        "chxchx-tui-{name}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("Cargo.toml"),
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    dir
}

fn app(name: &str) -> App {
    App::new(project(name)).expect("a plain folder is a project")
}

/// The screen as text, one line per row.
fn screen(app: &mut App, width: u16, height: u16) -> String {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal.draw(|frame| draw(frame, app)).unwrap();
    let buffer = terminal.backend().buffer();
    (0..height)
        .map(|y| {
            (0..width)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn press(app: &mut App, code: KeyCode) {
    handle_key(app, KeyEvent::from(code));
}

fn type_chars(app: &mut App, text: &str) {
    for c in text.chars() {
        press(app, KeyCode::Char(c));
    }
}

#[test]
fn the_home_page_shows_the_project_its_state_and_the_first_steps() {
    let mut app = app("home");
    let text = screen(&mut app, 110, 40);
    for expected in [
        "ChxChx",
        "Inicio",
        "Proyecto",
        "Estado",
        "Tecnología detectada",
        "Primeros pasos",
        "Prepara tu proyecto",
    ] {
        assert!(
            text.contains(expected),
            "{expected:?} missing from:\n{text}"
        );
    }
    assert!(text.contains("╗"), "a tall terminal shows the banner");
    assert!(text.contains("rust"), "the detected stack is listed");
    let short = screen(&mut app, 110, 24);
    assert!(!short.contains("╗"), "a short terminal drops the banner");
    assert!(short.contains("Primeros pasos"));
}

#[test]
fn a_terminal_that_is_too_small_asks_for_more_room() {
    let mut app = app("small");
    assert!(screen(&mut app, 40, 10).contains("Agranda la terminal"));
}

#[test]
fn tab_digits_and_clicks_move_between_every_page() {
    let mut app = app("nav");
    let _ = screen(&mut app, 110, 40);
    press(&mut app, KeyCode::Tab);
    assert_eq!(app.page, Page::Setup);
    press(&mut app, KeyCode::BackTab);
    assert_eq!(app.page, Page::Home);
    for (digit, page) in "12345678".chars().zip(Page::ALL) {
        press(&mut app, KeyCode::Char(digit));
        assert_eq!(app.page, page);
        let text = screen(&mut app, 110, 40);
        assert!(text.contains(page.title()), "{page:?}");
    }
    // A click on the "Catálogo" tab, wherever it was drawn.
    let rect = app
        .tab_hits
        .iter()
        .find(|(_, p)| *p == Page::Catalog)
        .unwrap()
        .0;
    handle_mouse(
        &mut app,
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: rect.x + 1,
            row: rect.y,
            modifiers: KeyModifiers::NONE,
        },
    );
    assert_eq!(app.page, Page::Catalog);
}

#[test]
fn help_opens_and_closes_and_q_quits_only_when_nothing_is_open() {
    let mut app = app("help");
    press(&mut app, KeyCode::Char('?'));
    assert!(screen(&mut app, 110, 40).contains("Atajos"));
    press(&mut app, KeyCode::Char('q'));
    assert!(!app.help && !app.quit, "q closes the help first");
    press(&mut app, KeyCode::Char('q'));
    assert!(app.quit);
}

#[test]
fn an_action_opens_a_preview_and_nothing_changes_until_it_is_confirmed() {
    let mut app = app("setup");
    press(&mut app, KeyCode::Char('2'));
    let text = screen(&mut app, 110, 40);
    assert!(text.contains("Preparar el proyecto") && text.contains("Dar confianza local"));
    press(&mut app, KeyCode::Enter);
    assert!(app.modal.is_some(), "Intro opens the preview");
    let text = screen(&mut app, 110, 40);
    assert!(
        text.contains("Preparar proyecto") && text.contains("Aplicar"),
        "{text}"
    );
    assert!(!app.path.join(".ai").exists(), "a preview writes nothing");
    press(&mut app, KeyCode::Esc);
    assert!(app.modal.is_none());
    assert!(
        !app.path.join(".ai").exists(),
        "cancelling writes nothing either"
    );
}

#[test]
fn trusting_a_project_previews_the_decision_before_saving_it() {
    let mut app = app("trust");
    press(&mut app, KeyCode::Char('2'));
    type_chars(&mut app, "jj");
    press(&mut app, KeyCode::Enter);
    let modal = app.modal.as_ref().expect("the trust preview");
    assert!(modal.title.contains("confianza"), "{}", modal.title);
    assert!(modal.lines.iter().any(|(_, l)| l.contains("no ejecuta")));
    assert!(!app.trusted(), "still not trusted");
}

#[test]
fn the_catalog_lists_skills_packs_and_mods_and_previews_enabling_one() {
    let mut app = app("catalog");
    press(&mut app, KeyCode::Char('3'));
    let text = screen(&mut app, 110, 40);
    assert!(
        text.contains("Skills") && text.contains("Packs") && text.contains("Mods"),
        "{text}"
    );
    assert!(
        text.contains("★") || text.contains("·"),
        "rows carry a marker"
    );
    press(&mut app, KeyCode::Right);
    assert_eq!(app.catalog_tab, CatalogTab::Packs);
    press(&mut app, KeyCode::Right);
    press(&mut app, KeyCode::Right);
    assert_eq!(app.catalog_tab, CatalogTab::Skills, "tabs wrap around");
    press(&mut app, KeyCode::Enter);
    assert!(app.modal.is_some(), "Intro previews the skill change");
}

#[test]
fn agents_need_trust_and_an_installation_before_they_open() {
    let mut app = app("agents");
    press(&mut app, KeyCode::Char('4'));
    let text = screen(&mut app, 110, 40);
    assert!(
        text.contains("Agentes") && text.contains("Detalle"),
        "{text}"
    );
    press(&mut app, KeyCode::Enter);
    assert!(
        app.launch.is_none(),
        "an untrusted project never launches an agent"
    );
    if !app.agents.agents.is_empty() {
        assert!(app.toast.is_some(), "the reason is shown");
    }
}

#[test]
fn health_and_projects_render_without_failing() {
    let mut app = app("health");
    press(&mut app, KeyCode::Char('7'));
    let text = screen(&mut app, 110, 40);
    assert!(
        text.contains("Salud") && text.contains("HERRAMIENTAS"),
        "{text}"
    );
    for _ in 0..50 {
        press(&mut app, KeyCode::Down);
    }
    let _ = screen(&mut app, 110, 40);
    press(&mut app, KeyCode::Char('8'));
    assert!(screen(&mut app, 110, 40).contains("Proyectos"));
    press(&mut app, KeyCode::Char('6'));
    assert!(screen(&mut app, 110, 40).contains("Conectar herramientas MCP"));
}

#[test]
fn git_reads_in_the_background_and_a_folder_without_git_says_so() {
    let mut app = app("git");
    press(&mut app, KeyCode::Char('5'));
    assert!(
        screen(&mut app, 110, 40).contains("Leyendo Git"),
        "while it loads"
    );
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    while app.git.is_none() && std::time::Instant::now() < deadline {
        app.poll();
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(matches!(app.git, Some(Ok(None))), "not a repository");
    assert!(screen(&mut app, 110, 40).contains("no es un repositorio Git"));
    assert!(app.busy.is_none(), "the spinner stops");
}

#[test]
fn finished_background_work_reports_back_and_closes_the_preview() {
    let mut app = app("done");
    press(&mut app, KeyCode::Char('2'));
    press(&mut app, KeyCode::Enter);
    assert!(app.modal.is_some());
    app.busy = Some("Aplicando".into());
    // What the worker thread sends when it is done.
    app.complete(Done::Applied(Ok("Listo".into())));
    assert!(app.modal.is_none() && app.busy.is_none());
    assert!(screen(&mut app, 110, 40).contains("Listo"));
    app.complete(Done::Applied(Err("falló".into())));
    assert!(screen(&mut app, 110, 40).contains("falló"));
}
