//! Native PTY/ConPTY adapter. Output stays in memory; bounded producer queue.
use portable_pty::{Child, CommandBuilder, MasterPty, PtySize, native_pty_system};
use std::io::{Read, Write};
use std::sync::mpsc::{Receiver, sync_channel};
use std::sync::{Arc, Mutex};

/// A terminal color as the program asked for it; the interface decides what it looks like.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Color {
    Default,
    Indexed(u8),
    Rgb(u8, u8, u8),
}

impl From<vt100::Color> for Color {
    fn from(color: vt100::Color) -> Self {
        match color {
            vt100::Color::Default => Color::Default,
            vt100::Color::Idx(index) => Color::Indexed(index),
            vt100::Color::Rgb(r, g, b) => Color::Rgb(r, g, b),
        }
    }
}

/// Consecutive cells of one row that share a style.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Run {
    pub text: String,
    pub fg: Color,
    pub bg: Color,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub inverse: bool,
    /// The cell under the cursor (always a run of exactly one cell).
    pub cursor: bool,
}

/// What is on screen: styled rows, the cursor and the modes keys depend on.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Snapshot {
    pub rows: Vec<Vec<Run>>,
    pub cols: u16,
    /// `(row, column)`; `None` when the program hid it or the view is scrolled back.
    pub cursor: Option<(u16, u16)>,
    /// Arrow keys send `ESC O A` instead of `ESC [ A` (full-screen programs ask for it).
    pub application_cursor: bool,
    /// Pasted text must be wrapped in `ESC [200~ … ESC [201~`.
    pub bracketed_paste: bool,
    /// Lines of history the view is scrolled back by.
    pub scrollback: usize,
}

pub struct Session {
    master: Option<Box<dyn MasterPty + Send>>,
    writer: Option<Arc<Mutex<Box<dyn Write + Send>>>>,
    child: Option<Box<dyn Child + Send + Sync>>,
    output: Receiver<Vec<u8>>,
    parser: vt100::Parser,
    dirty: bool,
    pub exited: bool,
}

impl std::fmt::Debug for Session {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TerminalSession")
            .field("exited", &self.exited)
            .finish_non_exhaustive()
    }
}

/// The screen as styled runs. Wide characters keep their cell; their blank continuation is skipped.
pub fn snapshot_of(screen: &vt100::Screen) -> Snapshot {
    let (rows, cols) = screen.size();
    let scrollback = screen.scrollback();
    let cursor = (!screen.hide_cursor() && scrollback == 0).then(|| screen.cursor_position());
    let mut lines = Vec::with_capacity(usize::from(rows));
    for row in 0..rows {
        let mut runs: Vec<Run> = Vec::new();
        for col in 0..cols {
            let Some(cell) = screen.cell(row, col) else {
                continue;
            };
            if cell.is_wide_continuation() {
                continue;
            }
            let text = if cell.has_contents() {
                cell.contents()
            } else {
                " "
            };
            let next = Run {
                text: text.to_owned(),
                fg: Color::from(cell.fgcolor()),
                bg: Color::from(cell.bgcolor()),
                bold: cell.bold(),
                italic: cell.italic(),
                underline: cell.underline(),
                inverse: cell.inverse(),
                cursor: cursor == Some((row, col)),
            };
            match runs.last_mut() {
                Some(run) if same_style(run, &next) => run.text.push_str(text),
                _ => runs.push(next),
            }
        }
        lines.push(runs);
    }
    Snapshot {
        rows: lines,
        cols,
        cursor,
        application_cursor: screen.application_cursor(),
        bracketed_paste: screen.bracketed_paste(),
        scrollback,
    }
}

fn same_style(a: &Run, b: &Run) -> bool {
    (
        a.fg,
        a.bg,
        a.bold,
        a.italic,
        a.underline,
        a.inverse,
        a.cursor,
    ) == (
        b.fg,
        b.bg,
        b.bold,
        b.italic,
        b.underline,
        b.inverse,
        b.cursor,
    )
}

impl Session {
    pub fn spawn(command: CommandBuilder) -> Result<Self, String> {
        let pair = native_pty_system()
            .openpty(PtySize {
                rows: 24,
                cols: 100,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| e.to_string())?;
        let mut reader = pair.master.try_clone_reader().map_err(|e| e.to_string())?;
        let writer = Arc::new(Mutex::new(
            pair.master.take_writer().map_err(|e| e.to_string())?,
        ));
        let responder = writer.clone();
        let (sender, output) = sync_channel(32);
        if let Err(error) = std::thread::Builder::new()
            .name("studio-pty-reader".into())
            .spawn(move || {
                let mut bytes = [0; 8192];
                let mut probe = Vec::new();
                loop {
                    match reader.read(&mut bytes) {
                        Ok(0) | Err(_) => break,
                        Ok(count) => {
                            probe.extend_from_slice(&bytes[..count]);
                            if probe.windows(4).any(|w| w == b"\x1b[6n")
                                && let Ok(mut writer) = responder.lock()
                            {
                                let _ = writer.write_all(b"\x1b[1;1R");
                                let _ = writer.flush();
                            }
                            if probe.len() > 3 {
                                probe.drain(..probe.len() - 3);
                            }
                            if sender.send(bytes[..count].to_vec()).is_err() {
                                break;
                            }
                        }
                    }
                }
            })
        {
            return Err(error.to_string());
        }
        let child = pair
            .slave
            .spawn_command(command)
            .map_err(|e| e.to_string())?;
        drop(pair.slave);
        Ok(Self {
            master: Some(pair.master),
            writer: Some(writer),
            child: Some(child),
            output,
            parser: vt100::Parser::new(24, 100, 1000),
            dirty: false,
            exited: false,
        })
    }

    pub fn send(&mut self, bytes: &[u8]) -> Result<(), String> {
        if self.exited {
            return Err("La terminal ya terminó".into());
        }
        if bytes.len() > 8192 {
            return Err("La entrada supera 8 KiB".into());
        }
        if self.parser.screen().scrollback() > 0 {
            self.parser.screen_mut().set_scrollback(0);
            self.dirty = true;
        }
        let mut writer = self
            .writer
            .as_ref()
            .ok_or("La terminal está cerrada")?
            .lock()
            .map_err(|_| "La entrada de terminal está bloqueada")?;
        writer
            .write_all(bytes)
            .and_then(|_| writer.flush())
            .map_err(|e| e.to_string())
    }

    /// Screen text, or `None` when nothing changed since the last poll, so the UI can
    /// skip rebuilding and re-laying out the terminal text.
    pub fn poll_changed(&mut self) -> Result<Option<String>, String> {
        let mut changed = std::mem::take(&mut self.dirty);
        for bytes in self.output.try_iter().take(32) {
            self.parser.process(&bytes);
            changed = true;
        }
        if let Some(child) = self.child.as_mut() {
            self.exited = child.try_wait().map_err(|e| e.to_string())?.is_some();
        }
        Ok(changed.then(|| self.parser.screen().contents()))
    }

    pub fn poll(&mut self) -> Result<String, String> {
        Ok(match self.poll_changed()? {
            Some(contents) => contents,
            None => self.parser.screen().contents(),
        })
    }

    /// The screen as styled runs (see [`snapshot_of`]).
    pub fn snapshot(&self) -> Snapshot {
        snapshot_of(self.parser.screen())
    }

    /// Scrolls the view back (`lines > 0`) or forward through the history; typing returns to the bottom.
    pub fn scroll(&mut self, lines: i32) {
        let screen = self.parser.screen_mut();
        let current = screen.scrollback() as i64;
        screen.set_scrollback((current + i64::from(lines)).clamp(0, 1000) as usize);
        self.dirty = true;
    }

    pub fn resize(&mut self, rows: u16, cols: u16) -> Result<(), String> {
        let rows = rows.clamp(4, 100);
        let cols = cols.clamp(20, 240);
        self.master
            .as_ref()
            .ok_or("La terminal está cerrada")?
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| e.to_string())?;
        self.parser.screen_mut().set_size(rows, cols);
        self.dirty = true;
        Ok(())
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        let master = self.master.take();
        let writer = self.writer.take();
        let child = self.child.take();
        let _ = std::thread::Builder::new()
            .name("studio-pty-reap".into())
            .spawn(move || {
                if let Some(mut child) = child {
                    let _ = child.kill();
                    drop(writer);
                    drop(master);
                    let _ = child.wait();
                }
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn screen(rows: u16, cols: u16, bytes: &[u8]) -> Snapshot {
        let mut parser = vt100::Parser::new(rows, cols, 100);
        parser.process(bytes);
        snapshot_of(parser.screen())
    }

    #[test]
    fn colors_and_styles_become_separate_runs_of_one_row() {
        let shot = screen(
            3,
            20,
            b"plano \x1b[31mrojo\x1b[0m \x1b[1;4;42mnegrita\x1b[0m",
        );

        let row = &shot.rows[0];
        let texts: Vec<_> = row.iter().map(|run| run.text.as_str()).collect();
        assert_eq!(&texts[..4], ["plano ", "rojo", " ", "negrita"]);
        assert_eq!(row[1].fg, Color::Indexed(1));
        assert!(row[3].bold && row[3].underline && row[3].bg == Color::Indexed(2));
        assert!(!row[0].bold && row[0].fg == Color::Default);
        assert_eq!(shot.rows.len(), 3);
        assert!(shot.rows[1].iter().all(|run| run.text.trim().is_empty()));
    }

    #[test]
    fn truecolor_and_inverse_are_kept_as_asked() {
        let shot = screen(2, 10, b"\x1b[38;2;10;20;30mA\x1b[7mB");

        assert_eq!(shot.rows[0][0].fg, Color::Rgb(10, 20, 30));
        assert!(shot.rows[0][1].inverse && !shot.rows[0][0].inverse);
    }

    #[test]
    fn the_cursor_modes_and_wide_characters_are_reported() {
        let shot = screen(3, 10, "日本x\x1b[?1h\x1b[?2004h".as_bytes());

        assert_eq!(shot.cursor, Some((0, 5))); // two wide cells take 4 columns, then `x`
        // The cell under the cursor is a run of its own, so the interface can draw it.
        let on_cursor: Vec<_> = shot.rows[0].iter().filter(|run| run.cursor).collect();
        assert_eq!(on_cursor.len(), 1);
        assert_eq!(on_cursor[0].text, " ");
        assert!(shot.application_cursor && shot.bracketed_paste);
        let first: String = shot.rows[0].iter().map(|run| run.text.as_str()).collect();
        assert!(
            first.starts_with("日本x"),
            "wide characters are one cell each: {first:?}"
        );
        let hidden = screen(2, 10, b"x\x1b[?25l");
        assert_eq!(hidden.cursor, None);
    }

    #[test]
    fn scrolling_back_shows_history_and_hides_the_cursor() {
        let mut parser = vt100::Parser::new(2, 10, 100);
        parser.process(b"uno\r\ndos\r\ntres\r\ncuatro");
        parser.screen_mut().set_scrollback(2);

        let shot = snapshot_of(parser.screen());

        assert_eq!(shot.scrollback, 2);
        assert_eq!(shot.cursor, None);
        assert!(shot.rows[0][0].text.starts_with("uno"));
    }

    /// A real program in a real PTY: it paints in color, waits for a line, and the screen shows both.
    #[cfg(unix)]
    #[test]
    fn a_program_can_be_driven_through_the_pty_and_its_screen_read_back_with_colors() {
        let mut command = CommandBuilder::new("/bin/sh");
        command.args([
            "-c",
            r#"printf '\033[31mrojo\033[0m listo\n'; read linea; printf 'recibido:%s\n' "$linea"; sleep 0.2"#,
        ]);
        command.env("TERM", "xterm-256color");
        let mut session = Session::spawn(command).expect("the PTY starts");

        let wait = |session: &mut Session, wanted: &str| {
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
            while std::time::Instant::now() < deadline {
                let _ = session.poll_changed();
                let shot = session.snapshot();
                let screen: String = shot
                    .rows
                    .iter()
                    .flat_map(|row| row.iter().map(|run| run.text.as_str()))
                    .collect();
                if screen.contains(wanted) {
                    return shot;
                }
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            panic!("the screen never showed {wanted:?}");
        };

        let first = wait(&mut session, "rojo");
        let colored = first.rows[0]
            .iter()
            .find(|run| run.text.starts_with("rojo"))
            .unwrap();
        assert_eq!(colored.fg, Color::Indexed(1));
        session.send(b"hola mundo\r").unwrap();
        let second = wait(&mut session, "recibido:hola mundo");
        assert_eq!(second.cols, 100);
    }

    #[test]
    fn native_pty_reads_a_short_lived_shell_and_resizes() {
        let mut command = CommandBuilder::new(if cfg!(windows) { "cmd.exe" } else { "/bin/sh" });
        command.args(if cfg!(windows) {
            vec!["/c", "echo CHXCHX_PTY_SMOKE"]
        } else {
            vec!["-c", "printf CHXCHX_PTY_SMOKE"]
        });
        let mut session = Session::spawn(command).expect("native PTY should start");
        session.resize(12, 80).unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        let mut observed = false;
        while std::time::Instant::now() < deadline {
            observed |= session.poll().unwrap().contains("CHXCHX_PTY_SMOKE");
            if observed && session.exited {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        assert!(observed, "PTY did not expose the command output");
        assert!(session.exited, "the short-lived shell should have finished");
    }

    #[test]
    fn interactive_shell_accepts_input_and_exits() {
        let mut command = CommandBuilder::new(if cfg!(windows) {
            "powershell.exe"
        } else {
            "/bin/sh"
        });
        if cfg!(windows) {
            command.args(["-NoLogo", "-NoProfile"]);
        }
        let mut session = Session::spawn(command).unwrap();
        session
            .send(if cfg!(windows) {
                b"Write-Output CHXCHX_INPUT_SMOKE\r"
            } else {
                b"printf '\\nCHXCHX_INPUT_SMOKE\\n'\r"
            })
            .unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        let mut observed = false;
        while std::time::Instant::now() < deadline {
            observed = session
                .poll()
                .unwrap()
                .lines()
                .any(|line| line.trim() == "CHXCHX_INPUT_SMOKE");
            if observed {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        assert!(
            observed,
            "the shell should execute the line, not only echo it"
        );
        session.send(b"exit\r").unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while std::time::Instant::now() < deadline && !session.exited {
            let _ = session.poll();
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        assert!(session.exited);
    }
}
