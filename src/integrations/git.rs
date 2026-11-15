//! The `git` process adapter. Never asks for credentials and never opens a window.
use std::io::{Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Commands that change the repository run one at a time, so they never fight over
/// `index.lock`. Read-only commands use `--no-optional-locks` and run freely.
static WRITERS: Mutex<()> = Mutex::new(());

#[derive(Clone, Debug)]
pub struct Run {
    pub args: Vec<String>,
    pub input: Option<Vec<u8>>,
    pub read_only: bool,
    pub max_output: usize,
    pub timeout: Duration,
    /// `-c key=value` settings applied before the command.
    pub config: Vec<String>,
}

impl Run {
    pub fn new<I, S>(args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            args: args.into_iter().map(Into::into).collect(),
            input: None,
            read_only: false,
            max_output: 8 * 1024 * 1024,
            timeout: Duration::from_secs(30),
            config: Vec::new(),
        }
    }

    pub fn read_only(mut self) -> Self {
        self.read_only = true;
        self
    }

    pub fn input(mut self, input: Vec<u8>) -> Self {
        self.input = Some(input);
        self
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn max_output(mut self, max_output: usize) -> Self {
        self.max_output = max_output;
        self
    }

    pub fn config(mut self, config: Vec<String>) -> Self {
        self.config = config;
        self
    }
}

#[derive(Clone, Debug, Default)]
pub struct Output {
    pub code: Option<i32>,
    pub stdout: Vec<u8>,
    pub stderr: String,
    pub truncated: bool,
    pub timed_out: bool,
}

impl Output {
    pub fn ok(&self) -> bool {
        self.code == Some(0) && !self.truncated && !self.timed_out
    }

    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.stdout).into_owned()
    }
}

/// Runs git in `root`. `Err` means git could not be started at all.
pub fn run(root: &Path, run: &Run) -> Result<Output, String> {
    let _writer = (!run.read_only).then(|| WRITERS.lock().unwrap_or_else(|e| e.into_inner()));
    let mut command = Command::new("git");
    command.current_dir(root);
    if run.read_only {
        command.arg("--no-optional-locks");
    }
    command.args(["-c", "core.quotepath=false"]);
    for setting in &run.config {
        command.args(["-c", setting]);
    }
    command
        .args(&run.args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GCM_INTERACTIVE", "never")
        .env("GIT_PAGER", "cat")
        // Stable English messages: failures are classified by their text.
        .env("LC_ALL", "C")
        .stdin(if run.input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    let mut child = command.spawn().map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            "No se encontró Git o no pudo iniciarse. Instálalo desde https://git-scm.com."
                .to_owned()
        } else {
            format!("No se pudo iniciar Git: {error}")
        }
    })?;
    if let (Some(input), Some(mut stdin)) = (run.input.clone(), child.stdin.take()) {
        std::thread::spawn(move || {
            let _ = stdin.write_all(&input);
        });
    }
    let limit = run.max_output;
    let mut stdout = child.stdout.take().expect("stdout is piped");
    let mut stderr = child.stderr.take().expect("stderr is piped");
    let out_reader = std::thread::spawn(move || {
        let mut buffer = Vec::new();
        let mut chunk = [0u8; 16 * 1024];
        let mut truncated = false;
        while let Ok(count) = stdout.read(&mut chunk) {
            if count == 0 {
                break;
            }
            if buffer.len() < limit {
                let room = (limit - buffer.len()).min(count);
                buffer.extend_from_slice(&chunk[..room]);
                truncated |= room < count;
            } else {
                truncated = true;
            }
        }
        (buffer, truncated)
    });
    let err_reader = std::thread::spawn(move || {
        let mut text = Vec::new();
        let _ = stderr.by_ref().take(64 * 1024).read_to_end(&mut text);
        let _ = std::io::copy(&mut stderr, &mut std::io::sink());
        String::from_utf8_lossy(&text).into_owned()
    });
    let deadline = Instant::now() + run.timeout;
    let mut timed_out = false;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) if Instant::now() >= deadline => {
                timed_out = true;
                let _ = child.kill();
                break child.wait().ok();
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(8)),
            Err(_) => break None,
        }
    };
    let (stdout, truncated) = out_reader.join().unwrap_or_default();
    let stderr = err_reader.join().unwrap_or_default();
    Ok(Output {
        code: status.and_then(|status| status.code()),
        stdout,
        stderr: if timed_out {
            "Git tardó demasiado y se detuvo.".to_owned()
        } else {
            stderr
        },
        truncated,
        timed_out,
    })
}
