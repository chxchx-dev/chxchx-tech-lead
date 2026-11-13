//! Native PTY/ConPTY adapter. Output stays in memory; bounded producer queue.
use portable_pty::{Child, CommandBuilder, MasterPty, PtySize, native_pty_system};
use std::io::{Read, Write};
use std::sync::mpsc::{Receiver, sync_channel};
use std::sync::{Arc, Mutex};

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
