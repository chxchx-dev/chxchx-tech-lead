use iced::widget::{button, column, container, pick_list, row, scrollable, text};
use iced::{Element, Fill, Subscription, Task};
use std::collections::VecDeque;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread;
use std::time::Duration;

const INSTALL_SCRIPT: &str = include_str!("../../packaging/install.sh");
const MAX_LOG_LINES: usize = 500;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Channel {
    Stable,
    Dev,
}

impl Channel {
    const ALL: [Self; 2] = [Self::Stable, Self::Dev];

    fn argument(self) -> &'static str {
        match self {
            Self::Stable => "stable",
            Self::Dev => "dev",
        }
    }
}

impl std::fmt::Display for Channel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Stable => "Estable",
            Self::Dev => "Preview (dev)",
        })
    }
}

#[derive(Clone, Copy, Debug)]
enum Action {
    DryRun,
    Install,
    Uninstall,
}

#[derive(Debug)]
enum WorkerEvent {
    Output(String),
    Finished(Action, Result<i32, String>),
}

#[derive(Debug, Clone)]
enum Message {
    ChannelChanged(Channel),
    DryRun,
    Install,
    Uninstall,
    Poll,
}

struct Installer {
    channel: Channel,
    running: bool,
    status: String,
    output: VecDeque<String>,
    receiver: Option<Receiver<WorkerEvent>>,
}

impl Installer {
    fn new() -> (Self, Task<Message>) {
        (
            Self {
                channel: Channel::Stable,
                running: false,
                status: "Listo para instalar".into(),
                output: VecDeque::new(),
                receiver: None,
            },
            Task::none(),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ChannelChanged(channel) if !self.running => self.channel = channel,
            Message::DryRun if !self.running => self.start(Action::DryRun),
            Message::Install if !self.running => self.start(Action::Install),
            Message::Uninstall if !self.running => self.start(Action::Uninstall),
            Message::Poll => self.drain_output(),
            _ => {}
        }
        Task::none()
    }

    fn start(&mut self, action: Action) {
        let (sender, receiver) = mpsc::channel();
        self.receiver = Some(receiver);
        self.running = true;
        self.output.clear();
        self.status = match action {
            Action::Install => format!("Instalando canal {}…", self.channel),
            Action::DryRun => format!("Verificando descargas del canal {}…", self.channel),
            Action::Uninstall => "Desinstalando ChxChx…".into(),
        };
        spawn_installer(action, self.channel, sender);
    }

    fn drain_output(&mut self) {
        let Some(receiver) = self.receiver.take() else {
            return;
        };
        loop {
            match receiver.try_recv() {
                Ok(WorkerEvent::Output(line)) => {
                    self.output.push_back(line);
                    while self.output.len() > MAX_LOG_LINES {
                        self.output.pop_front();
                    }
                }
                Ok(WorkerEvent::Finished(action, Ok(0))) => {
                    self.running = false;
                    self.receiver = None;
                    self.status = match action {
                        Action::Install => {
                            "Instalación terminada. Abre una terminal nueva para usar los comandos."
                                .into()
                        }
                        Action::DryRun => {
                            "Verificación terminada. No se realizaron cambios.".into()
                        }
                        Action::Uninstall => "Desinstalación terminada.".into(),
                    };
                    break;
                }
                Ok(WorkerEvent::Finished(action, Ok(code))) => {
                    self.running = false;
                    self.receiver = None;
                    self.status = format!("La operación terminó con código {code}.");
                    if matches!(action, Action::Install | Action::DryRun) {
                        self.status
                            .push_str(" Revisa el registro para ver el error.");
                    }
                    break;
                }
                Ok(WorkerEvent::Finished(_, Err(error))) => {
                    self.running = false;
                    self.receiver = None;
                    self.status = error;
                    break;
                }
                Err(TryRecvError::Empty) => {
                    self.receiver = Some(receiver);
                    break;
                }
                Err(TryRecvError::Disconnected) => {
                    self.running = false;
                    self.receiver = None;
                    self.status = "El instalador terminó sin devolver un resultado.".into();
                    break;
                }
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let channel_control: Element<'_, Message> = if self.running {
            text(format!("Canal: {}", self.channel)).into()
        } else {
            pick_list(Channel::ALL, Some(self.channel), Message::ChannelChanged).into()
        };
        let actions = row![
            button(text("Verificar sin instalar"))
                .on_press_maybe((!self.running).then_some(Message::DryRun)),
            button(text("Instalar ChxChx"))
                .on_press_maybe((!self.running).then_some(Message::Install)),
            button(text("Desinstalar"))
                .on_press_maybe((!self.running).then_some(Message::Uninstall)),
            channel_control,
        ]
        .spacing(12)
        .align_y(iced::Alignment::Center);
        let log = self.output.iter().cloned().collect::<Vec<_>>().join("\n");
        let content = column![
            text("Instalador de ChxChx").size(30),
            text("Instala Studio, los comandos de ChxChx y sus extras requeridos: Basic Memory y Serena. Se necesita conexión a Internet."),
            actions,
            text(&self.status).size(16),
            scrollable(container(text(log).font(iced::Font::MONOSPACE).size(12)).padding(12))
                .height(Fill),
        ]
        .spacing(18)
        .width(Fill)
        .height(Fill);

        container(content)
            .padding(26)
            .width(Fill)
            .height(Fill)
            .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        if self.running {
            iced::time::every(Duration::from_millis(120)).map(|_| Message::Poll)
        } else {
            Subscription::none()
        }
    }
}

pub fn run() -> iced::Result {
    iced::application(Installer::new, Installer::update, Installer::view)
        .title("Instalador de ChxChx")
        .window(iced::window::Settings {
            size: iced::Size::new(760.0, 560.0),
            min_size: Some(iced::Size::new(560.0, 420.0)),
            position: iced::window::Position::Centered,
            ..Default::default()
        })
        .subscription(Installer::subscription)
        .run()
}

fn spawn_installer(action: Action, channel: Channel, sender: Sender<WorkerEvent>) {
    thread::spawn(move || {
        let unique = format!(
            "chxchx-install-{}-{}.sh",
            std::process::id(),
            unique_suffix()
        );
        let script = std::env::temp_dir().join(unique);
        let result = std::fs::write(&script, INSTALL_SCRIPT)
            .map_err(|error| format!("No se pudo preparar el instalador: {error}"))
            .and_then(|()| run_script(&script, action, channel, &sender));
        let _ = std::fs::remove_file(script);
        let _ = sender.send(WorkerEvent::Finished(action, result));
    });
}

fn run_script(
    script: &std::path::Path,
    action: Action,
    channel: Channel,
    sender: &Sender<WorkerEvent>,
) -> Result<i32, String> {
    let mut command = Command::new("/bin/bash");
    command
        .arg(script)
        .env("CHXCHX_INSTALLER_GUI", "1")
        .env("UV_NO_PROGRESS", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    match action {
        Action::DryRun => {
            command.args(["--channel", channel.argument(), "--dry-run"]);
        }
        Action::Install => {
            command.args(["--channel", channel.argument()]);
        }
        Action::Uninstall => {
            command.arg("--uninstall");
        }
    }
    let mut child = command
        .spawn()
        .map_err(|error| format!("No se pudo iniciar bash: {error}"))?;
    let stdout = child.stdout.take().expect("stdout is piped");
    let stderr = child.stderr.take().expect("stderr is piped");
    let out_sender = sender.clone();
    let err_sender = sender.clone();
    let out_reader = thread::spawn(move || forward_lines(stdout, out_sender));
    let err_reader = thread::spawn(move || forward_lines(stderr, err_sender));
    let status = child
        .wait()
        .map_err(|error| format!("No se pudo esperar al instalador: {error}"));
    let _ = out_reader.join();
    let _ = err_reader.join();
    status.map(|status| status.code().unwrap_or(1))
}

fn forward_lines(reader: impl std::io::Read, sender: Sender<WorkerEvent>) {
    for line in BufReader::new(reader).lines() {
        match line {
            Ok(line) => {
                if sender.send(WorkerEvent::Output(line)).is_err() {
                    break;
                }
            }
            Err(error) => {
                let _ = sender.send(WorkerEvent::Output(format!(
                    "Error al leer salida: {error}"
                )));
                break;
            }
        }
    }
}

fn unique_suffix() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_nanos())
}
