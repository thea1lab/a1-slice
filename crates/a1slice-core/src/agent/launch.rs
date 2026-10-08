//! Which installed agents exist, and the command line for each one.

use std::path::Path;

use super::parse::js_len;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptionAgentId {
    Grok,
    Claude,
    Sol,
    Agy,
}

impl CaptionAgentId {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Grok => "grok",
            Self::Claude => "claude",
            Self::Sol => "sol",
            Self::Agy => "agy",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CaptionAgent {
    pub id: CaptionAgentId,
    pub label: &'static str,
    pub command: &'static str,
}

pub const CAPTION_AGENTS: [CaptionAgent; 4] = [
    CaptionAgent {
        id: CaptionAgentId::Grok,
        label: "Grok 4.7",
        command: "grok",
    },
    CaptionAgent {
        id: CaptionAgentId::Claude,
        label: "Sonnet 5.5",
        command: "claude",
    },
    CaptionAgent {
        id: CaptionAgentId::Sol,
        label: "Sol 5.6",
        command: "codex",
    },
    CaptionAgent {
        id: CaptionAgentId::Agy,
        label: "Gemini 3.8 Flash",
        command: "agy",
    },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CaptionAgentInfo {
    pub id: CaptionAgentId,
    pub label: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentLaunch {
    pub command: String,
    pub args: Vec<String>,
    pub stdin: Option<String>,
    pub stream_json: bool,
}

/// `lookup` returns the resolved path, or `None` when the command is not installed.
/// An empty string counts as missing, matching a shell lookup that prints nothing.
pub fn command_on_path<F, S>(command: &str, lookup: F) -> Option<S>
where
    F: Fn(&str) -> Option<S>,
    S: AsRef<str>,
{
    lookup(command).filter(|found| !found.as_ref().is_empty())
}

pub fn list_installed_agents<F, S>(lookup: F) -> Vec<CaptionAgentInfo>
where
    F: Fn(&str) -> Option<S>,
    S: AsRef<str>,
{
    CAPTION_AGENTS
        .iter()
        .filter_map(|agent| {
            command_on_path(agent.command, &lookup)?;
            Some(CaptionAgentInfo {
                id: agent.id,
                label: agent.label,
            })
        })
        .collect()
}

pub fn agent_launch(id: CaptionAgentId, prompt: &str, work_dir: &str) -> AgentLaunch {
    match id {
        CaptionAgentId::Grok => {
            let mut args = vec![
                "--no-plan".into(),
                "--model".into(),
                "grok-4.7".into(),
                "--reasoning-effort".into(),
                "low".into(),
                "--output-format".into(),
                "plain".into(),
                "--no-subagents".into(),
                "--disable-web-search".into(),
                "--cwd".into(),
                work_dir.into(),
            ];
            if js_len(prompt) < 100_000 {
                args.insert(0, prompt.to_string());
                args.insert(0, "-p".into());
            } else {
                let instructions = Path::new(work_dir).join("instructions.md");
                args.insert(0, instructions.to_string_lossy().into_owned());
                args.insert(0, "--prompt-file".into());
            }
            AgentLaunch {
                command: "grok".into(),
                args,
                stdin: None,
                stream_json: false,
            }
        }
        CaptionAgentId::Claude => AgentLaunch {
            command: "claude".into(),
            args: vec![
                "-p".into(),
                "--bare".into(),
                "--tools".into(),
                String::new(),
                "--no-session-persistence".into(),
                "--model".into(),
                "claude-sonnet-5-5".into(),
                "--effort".into(),
                "low".into(),
                "--output-format".into(),
                "text".into(),
                prompt.into(),
            ],
            stdin: None,
            stream_json: false,
        },
        CaptionAgentId::Sol => AgentLaunch {
            command: "codex".into(),
            args: vec![
                "exec".into(),
                "--skip-git-repo-check".into(),
                "--ephemeral".into(),
                "--color".into(),
                "never".into(),
                "-m".into(),
                "gpt-5.6-sol".into(),
                "-c".into(),
                "model_reasoning_effort=\"low\"".into(),
                "-s".into(),
                "read-only".into(),
                "-C".into(),
                work_dir.into(),
                "-".into(),
            ],
            stdin: Some(prompt.to_string()),
            stream_json: false,
        },
        CaptionAgentId::Agy => AgentLaunch {
            command: "agy".into(),
            args: vec![
                "--model".into(),
                "gemini-3.8-flash-low".into(),
                "--effort".into(),
                "low".into(),
                "--output-format".into(),
                "text".into(),
                "--disable-slash-commands".into(),
                "--print".into(),
                prompt.into(),
            ],
            stdin: None,
            stream_json: false,
        },
    }
}
