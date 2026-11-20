//! What the explorer's actions do to the disk (on a background thread) and, once they finish,
//! to everything that remembers a path: open tabs, expanded folders, the selection.
use super::*;
use crate::features::project_fs::{self, Trashed};
use std::collections::BTreeSet;

/// One piece of file work, with everything it needs so it can run off the UI thread.
#[derive(Clone, Debug)]
pub(super) enum Job {
    NewFile(PathBuf, String),
    NewFolder(PathBuf, String),
    Rename(PathBuf, String),
    Duplicate(Vec<PathBuf>),
    Paste(Clipboard, PathBuf),
    Delete(Vec<PathBuf>),
    Restore(Vec<Trashed>),
    Reveal(PathBuf),
}

#[derive(Debug)]
pub(super) enum Outcome {
    Created {
        path: PathBuf,
        directory: bool,
    },
    Renamed {
        from: PathBuf,
        to: PathBuf,
    },
    Pasted {
        moved: bool,
        from: Vec<PathBuf>,
        to: Vec<PathBuf>,
    },
    Duplicated(Vec<PathBuf>),
    /// What reached the trash, and why the rest did not.
    Deleted(Vec<Trashed>, Option<String>),
    Restored(Vec<PathBuf>, Option<String>),
    Revealed,
    Failed(String),
}

fn home() -> Result<PathBuf, String> {
    crate::features::project_profiles::application_home()
        .ok_or_else(|| "No se pudo localizar la carpeta de la aplicación".to_owned())
}

pub(super) fn execute(root: &Path, job: Job) -> Outcome {
    let fail = |error: String| Outcome::Failed(error);
    match job {
        Job::NewFile(parent, name) => match project_fs::create_file(root, &parent, &name) {
            Ok(path) => Outcome::Created {
                path,
                directory: false,
            },
            Err(error) => fail(error),
        },
        Job::NewFolder(parent, name) => match project_fs::create_folder(root, &parent, &name) {
            Ok(path) => Outcome::Created {
                path,
                directory: true,
            },
            Err(error) => fail(error),
        },
        Job::Rename(path, name) => match project_fs::rename(root, &path, &name) {
            Ok(to) => Outcome::Renamed { from: path, to },
            Err(error) => fail(error),
        },
        Job::Duplicate(items) => {
            let mut made = Vec::new();
            for item in &items {
                match project_fs::duplicate(root, item) {
                    Ok(path) => made.push(path),
                    Err(error) => return fail(error),
                }
            }
            Outcome::Duplicated(made)
        }
        Job::Paste(clipboard, destination) => {
            let result = if clipboard.cut {
                project_fs::move_into(root, &clipboard.paths, &destination)
            } else {
                project_fs::copy_into(root, &clipboard.paths, &destination)
            };
            match result {
                Ok(to) => Outcome::Pasted {
                    moved: clipboard.cut,
                    from: clipboard.paths,
                    to,
                },
                Err(error) => fail(error),
            }
        }
        Job::Delete(items) => {
            let home = match home() {
                Ok(home) => home,
                Err(error) => return fail(error),
            };
            let (mut done, mut problem) = (Vec::new(), None);
            for item in &items {
                match project_fs::trash(root, &home, item) {
                    Ok(trashed) => done.push(trashed),
                    Err(error) => {
                        problem = Some(error);
                        break;
                    }
                }
            }
            Outcome::Deleted(done, problem)
        }
        Job::Restore(batch) => {
            let (mut back, mut problem) = (Vec::new(), None);
            for trashed in &batch {
                match project_fs::restore(root, trashed) {
                    Ok(path) => back.push(path),
                    Err(error) => {
                        problem = Some(error);
                        break;
                    }
                }
            }
            Outcome::Restored(back, problem)
        }
        Job::Reveal(path) => match crate::integrations::system_open::reveal(&path) {
            Ok(()) => Outcome::Revealed,
            Err(error) => fail(error),
        },
    }
}

pub(super) fn spawn(state: &mut Dashboard, job: Job) -> Task<Message> {
    let root = state.root.clone();
    jobs::run(state, move || execute(&root, job), finished)
}

fn parents(paths: &[PathBuf]) -> BTreeSet<PathBuf> {
    paths
        .iter()
        .map(|path| path.parent().map(Path::to_path_buf).unwrap_or_default())
        .collect()
}

/// Lists the folders again so the tree shows what is on disk now (and the Git badges follow).
fn relist(state: &mut Dashboard, folders: BTreeSet<PathBuf>) -> Task<Message> {
    let root = state.root.clone();
    let mut tasks: Vec<Task<Message>> = folders
        .into_iter()
        .filter(|folder| folder.as_os_str().is_empty() || state.editor.expanded.contains(folder))
        .map(|folder| editor::list(root.clone(), folder))
        .collect();
    tasks.push(git::refresh(state));
    Task::batch(tasks)
}

/// Every folder the tree has open, for a full refresh.
pub(super) fn refresh(state: &mut Dashboard) -> Task<Message> {
    let mut folders: BTreeSet<PathBuf> = state.editor.expanded.iter().take(200).cloned().collect();
    folders.insert(PathBuf::new());
    relist(state, folders)
}

fn plural(count: usize, one: &str, many: &str) -> String {
    if count == 1 {
        format!("1 {one}")
    } else {
        format!("{count} {many}")
    }
}

fn select(state: &mut Dashboard, paths: &[PathBuf]) {
    state.explorer.selection = paths.iter().cloned().collect();
    state.explorer.anchor = paths.last().cloned();
}

pub(super) fn finished(state: &mut Dashboard, outcome: Outcome) -> Task<Message> {
    match outcome {
        Outcome::Failed(error) => {
            if let Some(edit) = state.explorer.edit.as_mut() {
                edit.error = Some(error);
            } else {
                state.message = error;
            }
            Task::none()
        }
        Outcome::Revealed => Task::none(),
        Outcome::Created { path, directory } => {
            state.explorer.edit = None;
            select(state, std::slice::from_ref(&path));
            let parent = parents(std::slice::from_ref(&path));
            let list = relist(state, parent);
            if directory {
                state.editor.expanded.insert(path);
                list
            } else {
                Task::batch([list, Task::done(Message::Editor(editor::Event::Open(path)))])
            }
        }
        Outcome::Renamed { from, to } => {
            state.explorer.edit = None;
            state.editor.retarget(&from, &to);
            select(state, std::slice::from_ref(&to));
            let parent = parents(&[from, to]);
            relist(state, parent)
        }
        Outcome::Pasted { moved, from, to } => {
            if moved {
                for (old, new) in from.iter().zip(&to) {
                    state.editor.retarget(old, new);
                }
                state.explorer.clipboard = None;
            }
            state.message = format!(
                "{} {}.",
                plural(to.len(), "elemento", "elementos"),
                if moved { "movidos" } else { "pegados" }
            )
            .replacen("1 elemento movidos", "1 elemento movido", 1)
            .replacen("1 elemento pegados", "1 elemento pegado", 1);
            select(state, &to);
            let mut folders = parents(&to);
            if moved {
                folders.extend(parents(&from));
            }
            relist(state, folders)
        }
        Outcome::Duplicated(made) => {
            select(state, &made);
            let folders = parents(&made);
            relist(state, folders)
        }
        Outcome::Deleted(done, problem) => {
            state.explorer.confirm = None;
            let originals: Vec<PathBuf> = done.iter().map(|t| t.original.clone()).collect();
            for original in &originals {
                state.editor.forget(original);
            }
            state.explorer.selection.clear();
            state.explorer.anchor = None;
            if !done.is_empty() {
                state.message = format!(
                    "{} movidos a la papelera; puedes deshacerlo desde el explorador.",
                    plural(done.len(), "elemento", "elementos")
                )
                .replacen("1 elemento movidos", "1 elemento movido", 1);
                remember_deletion(&mut state.explorer, done);
            }
            if let Some(error) = problem {
                state.message = format!("No se pudo eliminar todo: {error}");
            }
            let folders = parents(&originals);
            relist(state, folders)
        }
        Outcome::Restored(back, problem) => {
            select(state, &back);
            state.message = match problem {
                Some(error) => format!("No se pudo restaurar todo: {error}"),
                None => format!(
                    "{} restaurados.",
                    plural(back.len(), "elemento", "elementos")
                )
                .replacen("1 elemento restaurados", "1 elemento restaurado", 1),
            };
            let folders = parents(&back);
            relist(state, folders)
        }
    }
}
