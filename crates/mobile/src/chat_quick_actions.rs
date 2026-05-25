//! One-tap prompts for common coding-agent workflows.

use dioxus::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QuickAction {
    pub label: &'static str,
    pub prompt: &'static str,
}

pub const QUICK_ACTIONS: &[QuickAction] = &[
    QuickAction {
        label: "Plan",
        prompt: "Plan mode: analyze the active workspace and propose next steps. Do not assume tools ran — only reason from context.",
    },
    QuickAction {
        label: "Termux pwd",
        prompt: "Run pwd and ls -la in the active Termux workspace via exec_shell and summarize the environment.",
    },
    QuickAction {
        label: "Git status",
        prompt: "Run git status in the active workspace and summarize branch, staged changes, and risks.",
    },
    QuickAction {
        label: "Run tests",
        prompt: "Run the project's test command (prefer cargo test --workspace if Rust). Summarize failures with file paths.",
    },
    QuickAction {
        label: "Read structure",
        prompt: "List the top-level project structure and identify the main entry points, build system, and config files.",
    },
    QuickAction {
        label: "Fix diagnostics",
        prompt: "Review the latest diagnostics context and propose minimal fixes. Prefer small safe patches.",
    },
    QuickAction {
        label: "Open on PC",
        prompt: "If a PC Host workspace is active, use open_path on the project root. Otherwise say Termux is the active executor and open the project path with list_dir.",
    },
];

pub fn chat_quick_actions_bar(on_select: EventHandler<String>) -> Element {
    rsx! {
        div {
            display: "flex",
            gap: "6px",
            overflow_x: "auto",
            margin_bottom: "8px",
            padding_bottom: "4px",

            for action in QUICK_ACTIONS {
                button {
                    flex_shrink: "0",
                    background_color: "#1f2937",
                    color: "#e5e7eb",
                    border: "1px solid #4b5563",
                    border_radius: "999px",
                    padding: "6px 10px",
                    font_size: "11px",
                    white_space: "nowrap",
                    onclick: {
                        let prompt = action.prompt.to_string();
                        move |_| on_select.call(prompt.clone())
                    },
                    "{action.label}"
                }
            }
        }
    }
}
