//! What the store's reads find on disk: the folders, each folder's transcripts with whose
//! each one is, a subagent's type from its meta file, and the name Claude Code gives a
//! path's folder. Each case lays a store out in a scratch directory of its own, as Claude
//! Code lays one out, and asserts on the whole of what the read returns.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use super::{
    AgentType, Folder, Found, META_BYTES_MAX, Thread, folder_name_of, folders, projects_folder,
    transcripts_in,
};

static NEXT_DIRECTORY: AtomicUsize = AtomicUsize::new(0);

/// A directory of its own for one case, removed when the case ends.
struct Scratch {
    root: PathBuf,
}

impl Scratch {
    fn new() -> Self {
        let index = NEXT_DIRECTORY.fetch_add(1, Ordering::SeqCst);
        let name = format!("claude-code-transcript-{}-{index}", std::process::id());
        let root = std::env::temp_dir().join(name);
        fs::create_dir_all(&root).unwrap();
        Self { root }
    }

    fn write(&self, name: &str, text: &str) -> PathBuf {
        let path = self.root.join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, text).unwrap();
        path
    }

    fn folder(&self, name: &str) -> Folder {
        Folder {
            name: name.to_owned(),
            path: self.root.join(name),
        }
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).unwrap();
    }
}

fn subagent(path: PathBuf, session: &str, agent_id: &str, agent_type: AgentType) -> Found {
    Found {
        path,
        session: session.to_owned(),
        thread: Thread::Subagent {
            agent_id: agent_id.to_owned(),
            agent_type,
        },
    }
}

mod given_a_home {
    use super::*;

    #[test]
    fn when_its_projects_folder_is_asked_then_it_is_beneath_dot_claude() {
        let expected = Path::new("/home/someone/.claude/projects");
        assert_eq!(projects_folder(Path::new("/home/someone")), expected);
    }
}

mod given_a_path_a_session_ran_in {
    use super::*;

    #[test]
    fn when_its_folder_is_named_then_every_character_but_letters_and_digits_is_a_hyphen() {
        let cases: [(&str, &str); 3] = [
            (
                "/home/admin/projects/excerpt",
                "-home-admin-projects-excerpt",
            ),
            ("/tmp/nix-shell.NAXnJd/x_y", "-tmp-nix-shell-NAXnJd-x-y"),
            ("/home/admin/é", "-home-admin--"),
        ];
        for (path, expected) in cases {
            assert_eq!(folder_name_of(path), expected, "{path}");
        }
    }
}

mod given_a_store {
    use super::*;

    #[test]
    fn when_its_folders_are_listed_then_only_folders_come_in_name_order() {
        let scratch = Scratch::new();
        scratch.write("-home-b/s.jsonl", "");
        scratch.write("-home-a/s.jsonl", "");
        scratch.write("stray.jsonl", "");
        let expected = vec![scratch.folder("-home-a"), scratch.folder("-home-b")];
        assert_eq!(folders(&scratch.root).unwrap(), expected);
    }

    #[test]
    fn when_a_folders_transcripts_are_read_then_each_thread_is_named() {
        let scratch = Scratch::new();
        let main = scratch.write("-home-a/s1.jsonl", "");
        let agent = scratch.write("-home-a/s1/subagents/agent-a1.jsonl", "");
        scratch.write(
            "-home-a/s1/subagents/agent-a1.meta.json",
            r#"{"agentType":"Explore"}"#,
        );
        let workflow = scratch.write("-home-a/s1/subagents/workflows/wf_1/agent-a2.jsonl", "");
        scratch.write("-home-a/s1/tool-results/toolu_01.txt", "saved");
        // Paths compare by component, so `s1/…` sorts before `s1.jsonl`.
        let expected = vec![
            subagent(agent, "s1", "a1", AgentType::Named("Explore".to_owned())),
            subagent(workflow, "s1", "a2", AgentType::NoMeta),
            Found {
                path: main,
                session: "s1".to_owned(),
                thread: Thread::Main,
            },
        ];
        assert_eq!(
            transcripts_in(&scratch.folder("-home-a")).unwrap(),
            expected
        );
    }

    #[test]
    fn when_a_subagent_sits_past_the_depth_bound_then_it_is_not_found() {
        let scratch = Scratch::new();
        scratch.write("-home-a/s1/subagents/1/2/3/4/5/agent-a1.jsonl", "");
        let shallow = scratch.write("-home-a/s1/subagents/1/2/3/4/agent-a2.jsonl", "");
        let expected = vec![subagent(shallow, "s1", "a2", AgentType::NoMeta)];
        assert_eq!(
            transcripts_in(&scratch.folder("-home-a")).unwrap(),
            expected
        );
    }

    #[test]
    fn when_a_meta_file_names_no_type_or_fails_its_check_then_the_type_says_so() {
        let oversized = format!(
            r#"{{"agentType":"{}"}}"#,
            "x".repeat(usize::try_from(META_BYTES_MAX).unwrap())
        );
        let cases: [(&str, AgentType); 4] = [
            (r#"{"description":"review"}"#, AgentType::Unnamed),
            (r#"{"agentType":7}"#, AgentType::Malformed),
            ("not json", AgentType::Malformed),
            (oversized.as_str(), AgentType::Malformed),
        ];
        for (meta, expected) in cases {
            let scratch = Scratch::new();
            let agent = scratch.write("-home-a/s1/subagents/agent-a1.jsonl", "");
            scratch.write("-home-a/s1/subagents/agent-a1.meta.json", meta);
            let found = vec![subagent(agent, "s1", "a1", expected)];
            assert_eq!(
                transcripts_in(&scratch.folder("-home-a")).unwrap(),
                found,
                "{meta:.40}"
            );
        }
    }
}
