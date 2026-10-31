//! Where Claude Code keeps its transcripts and whose each one is, read off disk under
//! bounds.
//!
//! The layout is the one this machine's store holds (checked 2026-10-30): a folder per
//! path a session ran in, beneath the projects folder, named for that path with every
//! character but an ASCII letter or digit replaced by a hyphen; a session's main thread at
//! `<folder>/<session>.jsonl`; and its subagents' transcripts at
//! `<folder>/<session>/subagents/agent-<id>.jsonl`, a workflow's two folders further down
//! under `workflows/`, each beside an `agent-<id>.meta.json` naming the subagent's type
//! under `agentType`.

use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use serde_json::Value;

/// Where Claude Code keeps its transcripts beneath the home.
pub const PROJECTS_UNDER_HOME: [&str; 2] = [".claude", "projects"];

/// The most entries one folder is read for.
pub const FOLDER_ENTRIES_MAX: usize = 1_000_000;

/// The most transcripts one folder of the store holds, its subagents' included.
pub const TRANSCRIPTS_PER_FOLDER_MAX: usize = 100_000;

/// The deepest a subagent's transcript sits below its session's `subagents` folder; a
/// workflow's sit two levels down.
pub const SUBAGENT_DEPTH_MAX: usize = 4;

/// The largest meta file read.
pub const META_BYTES_MAX: u64 = 64 * 1024;

const SUBAGENTS_FOLDER: &str = "subagents";
const TRANSCRIPT_EXTENSION: &str = "jsonl";
const META_EXTENSION: &str = "meta.json";
const AGENT_PREFIX: &str = "agent-";
const AGENT_TYPE_KEY: &str = "agentType";
const FOLDER_DIVIDER: char = '-';

/// A folder of the store: the sessions run at one path.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Folder {
    /// The folder's name, which [`folder_name_of`] makes of the path.
    pub name: String,
    /// Where the folder stands.
    pub path: PathBuf,
}

/// A transcript, and whose it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Found {
    /// Where the transcript stands.
    pub path: PathBuf,
    /// The session it belongs to.
    pub session: String,
    /// Whether it is the session's main thread or a subagent's.
    pub thread: Thread,
}

/// Whose thread a transcript records.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Thread {
    /// The session's own.
    Main,
    /// A subagent's the session started.
    Subagent {
        /// The id the subagent's records carry under `agentId`.
        agent_id: String,
        /// The subagent's type, as its meta file names it.
        agent_type: AgentType,
    },
}

/// What a subagent's meta file says of its type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentType {
    /// The type the file names, such as `Explore`.
    Named(String),
    /// The file names no type.
    Unnamed,
    /// No meta file stands beside the transcript.
    NoMeta,
    /// The file is larger than [`META_BYTES_MAX`], or no JSON object.
    Malformed,
    /// The file could not be read.
    Unread(io::ErrorKind),
}

/// Why the store could not be listed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreFault {
    /// A folder could not be listed.
    Unlisted {
        /// The folder.
        path: PathBuf,
        /// What listing it failed with.
        kind: io::ErrorKind,
    },
    /// A folder holds more than [`FOLDER_ENTRIES_MAX`] entries.
    TooManyEntries {
        /// The folder.
        path: PathBuf,
    },
    /// A folder of the store holds more than [`TRANSCRIPTS_PER_FOLDER_MAX`] transcripts.
    TooManyTranscripts {
        /// The folder.
        path: PathBuf,
    },
}

/// The projects folder beneath `home`.
///
/// # Panics
///
/// When the folder joined stands outside `home`, which no home can cause.
#[must_use]
pub fn projects_folder(home: &Path) -> PathBuf {
    let mut folder = home.to_path_buf();
    for part in PROJECTS_UNDER_HOME {
        folder.push(part);
    }
    assert!(
        folder.starts_with(home),
        "the projects folder stands beneath the home"
    );
    folder
}

/// The name Claude Code gives the folder of the sessions run at `path`.
///
/// # Panics
///
/// When the name is not one character for each of the path's, which no path can cause.
#[must_use]
pub fn folder_name_of(path: &str) -> String {
    let name: String = path
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                FOLDER_DIVIDER
            }
        })
        .collect();
    assert_eq!(
        name.chars().count(),
        path.chars().count(),
        "each character of the path stands for one of the name"
    );
    name
}

/// The folders of the store, in name order. An entry that is no folder, or whose name is
/// no UTF-8 text, is passed over.
///
/// # Errors
///
/// A [`StoreFault`] where the projects folder cannot be listed or holds too many entries.
///
/// # Panics
///
/// When the folders leave name order, which no listing can cause.
pub fn folders(projects: &Path) -> Result<Vec<Folder>, StoreFault> {
    let mut folders = Vec::new();
    for entry in entries_of(projects)? {
        if !entry.is_folder {
            continue;
        }
        let Some(name) = name_of(&entry.path) else {
            continue;
        };
        folders.push(Folder {
            name,
            path: entry.path,
        });
    }
    assert!(folders.is_sorted(), "the folders keep the listing's order");
    Ok(folders)
}

/// The transcripts a folder of the store holds, in path order: each session's main
/// thread, and each of its subagents' with the type its meta file names.
///
/// # Errors
///
/// A [`StoreFault`] where a folder beneath it cannot be listed, or a bound is passed.
pub fn transcripts_in(folder: &Folder) -> Result<Vec<Found>, StoreFault> {
    let mut found = Vec::new();
    for entry in entries_of(&folder.path)? {
        if entry.is_folder {
            let Some(session) = name_of(&entry.path) else {
                continue;
            };
            let subagents = entry.path.join(SUBAGENTS_FOLDER);
            if subagents.is_dir() {
                for path in subagent_paths(&subagents)? {
                    let Some(subagent) = subagent_found(path, &session) else {
                        continue;
                    };
                    found.push(subagent);
                }
            }
        } else if is_transcript(&entry.path) {
            let Some(session) = stem_of(&entry.path) else {
                continue;
            };
            found.push(Found {
                path: entry.path,
                session,
                thread: Thread::Main,
            });
        }
        if found.len() > TRANSCRIPTS_PER_FOLDER_MAX {
            return Err(StoreFault::TooManyTranscripts {
                path: folder.path.clone(),
            });
        }
    }
    found.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(found)
}

/// A subagent's transcript, its id taken from its file's name and its type from its
/// meta file; none where its file's name is no UTF-8 text.
fn subagent_found(path: PathBuf, session: &str) -> Option<Found> {
    let stem = stem_of(&path)?;
    let agent_id = match stem.strip_prefix(AGENT_PREFIX) {
        Some(id) => id.to_owned(),
        None => stem,
    };
    let agent_type = agent_type_of(&path);
    Some(Found {
        path,
        session: session.to_owned(),
        thread: Thread::Subagent {
            agent_id,
            agent_type,
        },
    })
}

/// The transcripts beneath a session's `subagents` folder, at most
/// [`SUBAGENT_DEPTH_MAX`] levels down.
fn subagent_paths(subagents: &Path) -> Result<Vec<PathBuf>, StoreFault> {
    let mut paths = Vec::new();
    let mut pending = vec![(subagents.to_path_buf(), 0_usize)];
    while let Some((folder, depth)) = pending.pop() {
        assert!(depth <= SUBAGENT_DEPTH_MAX, "the walk stops at its depth");
        for entry in entries_of(&folder)? {
            if entry.is_folder {
                if depth < SUBAGENT_DEPTH_MAX {
                    let Some(deeper) = depth.checked_add(1) else {
                        unreachable!("the depth stays below its bound")
                    };
                    pending.push((entry.path, deeper));
                }
            } else if is_transcript(&entry.path) {
                paths.push(entry.path);
            }
        }
        if paths.len() > TRANSCRIPTS_PER_FOLDER_MAX {
            return Err(StoreFault::TooManyTranscripts {
                path: subagents.to_path_buf(),
            });
        }
    }
    Ok(paths)
}

/// What the meta file beside a subagent's transcript names of its type.
fn agent_type_of(transcript: &Path) -> AgentType {
    let meta = transcript.with_extension(META_EXTENSION);
    let file = match File::open(&meta) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return AgentType::NoMeta,
        Err(error) => return AgentType::Unread(error.kind()),
    };
    let Some(take_bytes) = META_BYTES_MAX.checked_add(1) else {
        unreachable!("the meta bound and one byte past it fit a u64")
    };
    let mut text = String::new();
    if let Err(error) = file.take(take_bytes).read_to_string(&mut text) {
        if error.kind() == io::ErrorKind::InvalidData {
            return AgentType::Malformed;
        }
        return AgentType::Unread(error.kind());
    }
    let Ok(length) = u64::try_from(text.len()) else {
        unreachable!("a bounded file's length fits a u64")
    };
    if length > META_BYTES_MAX {
        return AgentType::Malformed;
    }
    match serde_json::from_str::<Value>(&text) {
        Ok(Value::Object(meta)) => match meta.get(AGENT_TYPE_KEY) {
            Some(Value::String(name)) if !name.is_empty() => AgentType::Named(name.clone()),
            None | Some(Value::Null) => AgentType::Unnamed,
            Some(_) => AgentType::Malformed,
        },
        Ok(_) | Err(_) => AgentType::Malformed,
    }
}

/// An entry of a folder: its path, and whether it is a folder itself.
struct Entry {
    path: PathBuf,
    is_folder: bool,
}

/// The entries of a folder in path order, at most [`FOLDER_ENTRIES_MAX`] of them. A
/// symbolic link is no folder, so the walk never follows one.
fn entries_of(folder: &Path) -> Result<Vec<Entry>, StoreFault> {
    let unlisted = |error: io::Error| StoreFault::Unlisted {
        path: folder.to_path_buf(),
        kind: error.kind(),
    };
    let listing = fs::read_dir(folder).map_err(unlisted)?;
    let mut entries = Vec::new();
    for entry in listing {
        if entries.len() == FOLDER_ENTRIES_MAX {
            return Err(StoreFault::TooManyEntries {
                path: folder.to_path_buf(),
            });
        }
        let entry = entry.map_err(unlisted)?;
        let kind = entry.file_type().map_err(unlisted)?;
        entries.push(Entry {
            path: entry.path(),
            is_folder: kind.is_dir(),
        });
    }
    entries.sort_by(|left, right| left.path.cmp(&right.path));
    assert!(
        entries.len() <= FOLDER_ENTRIES_MAX,
        "the listing stops at its bound"
    );
    Ok(entries)
}

fn is_transcript(path: &Path) -> bool {
    path.extension().and_then(|extension| extension.to_str()) == Some(TRANSCRIPT_EXTENSION)
}

fn name_of(path: &Path) -> Option<String> {
    let name = path.file_name()?;
    name.to_str().map(str::to_owned)
}

fn stem_of(path: &Path) -> Option<String> {
    let stem = path.file_stem()?;
    stem.to_str().map(str::to_owned)
}

#[cfg(test)]
mod tests;
