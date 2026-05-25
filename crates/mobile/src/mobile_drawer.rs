use dioxus::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CockpitSection {
    Chat,
    PcHost,
    Files,
    Snapshots,
    Diagnostics,
    Terminal,
    Approvals,
    Mcp,
    Skills,
    Git,
    Tasks,
    Settings,
}

impl CockpitSection {
    pub fn title(self) -> &'static str {
        match self {
            CockpitSection::Chat => "Chat",
            CockpitSection::PcHost => "PC Host",
            CockpitSection::Files => "Files",
            CockpitSection::Snapshots => "Snapshots",
            CockpitSection::Diagnostics => "Diagnostics",
            CockpitSection::Terminal => "Terminal",
            CockpitSection::Approvals => "Approvals",
            CockpitSection::Mcp => "MCP",
            CockpitSection::Skills => "Skills",
            CockpitSection::Git => "Git & GitHub",
            CockpitSection::Tasks => "Tasks",
            CockpitSection::Settings => "Settings",
        }
    }

    pub fn subtitle(self) -> &'static str {
        match self {
            CockpitSection::Chat => "Main AI conversation and tool timeline",
            CockpitSection::PcHost => "Pairing, online status, workspaces",
            CockpitSection::Files => "Project tree, open files, diffs",
            CockpitSection::Snapshots => "Safety points and rollback readiness",
            CockpitSection::Diagnostics => "Post-edit errors and warnings",
            CockpitSection::Terminal => "PC / Termux command output",
            CockpitSection::Approvals => "Tool calls waiting for confirmation",
            CockpitSection::Mcp => "MCP server status, tools, config",
            CockpitSection::Skills => "Skill bundles: enable/disable",
            CockpitSection::Git => "Status, commits, push, pull, PRs",
            CockpitSection::Tasks => "Background tasks, build jobs, test runs",
            CockpitSection::Settings => "DeepSeek API, GitHub, disks, security",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MobileChromeSummary {
    pub api_configured: bool,
    pub pc_label: String,
    pub pc_online: bool,
    pub active_project_title: String,
    pub active_project_subtitle: String,
    pub pending_approvals: usize,
    pub running_tasks: usize,
    pub diagnostics_errors: usize,
    pub diagnostics_warnings: usize,
    pub dirty_files: usize,
    pub native_waiting: bool,
}

impl Default for MobileChromeSummary {
    fn default() -> Self {
        Self {
            api_configured: false,
            pc_label: "PC SETUP".to_string(),
            pc_online: false,
            active_project_title: "Local workspace".to_string(),
            active_project_subtitle: "No PC workspace is active yet".to_string(),
            pending_approvals: 0,
            running_tasks: 0,
            diagnostics_errors: 0,
            diagnostics_warnings: 0,
            dirty_files: 0,
            native_waiting: false,
        }
    }
}

impl MobileChromeSummary {
    pub fn api_chip_label(&self) -> &'static str {
        if self.api_configured {
            "API OK"
        } else {
            "NO API"
        }
    }

    pub fn api_chip_colors(&self) -> (&'static str, &'static str, &'static str) {
        if self.api_configured {
            ("#052e2b", "#10b981", "#a7f3d0")
        } else {
            ("#3f1d1d", "#ef4444", "#fecaca")
        }
    }

    pub fn pc_chip_colors(&self) -> (&'static str, &'static str, &'static str) {
        if self.pc_online {
            ("#052e2b", "#10b981", "#a7f3d0")
        } else if self.pc_label.contains("ERR") || self.pc_label.contains("OFF") {
            ("#3f1d1d", "#ef4444", "#fecaca")
        } else {
            ("#1e3a8a", "#3b82f6", "#bfdbfe")
        }
    }

    pub fn badge_for(&self, section: CockpitSection) -> Option<String> {
        match section {
            CockpitSection::PcHost => Some(self.pc_label.clone()),
            CockpitSection::Approvals if self.pending_approvals > 0 => {
                Some(self.pending_approvals.to_string())
            }
            CockpitSection::Diagnostics if self.diagnostics_errors > 0 => {
                Some(format!("ERR {}", self.diagnostics_errors))
            }
            CockpitSection::Diagnostics if self.diagnostics_warnings > 0 => {
                Some(format!("WARN {}", self.diagnostics_warnings))
            }
            CockpitSection::Git if self.dirty_files > 0 => Some(self.dirty_files.to_string()),
            CockpitSection::Tasks if self.running_tasks > 0 => {
                Some(format!("RUN {}", self.running_tasks))
            }
            CockpitSection::Chat if self.native_waiting => Some("WAIT".to_string()),
            _ => None,
        }
    }

    pub fn compact_badge_for(&self, section: CockpitSection) -> Option<String> {
        match section {
            CockpitSection::PcHost if self.pc_online => Some("ON".to_string()),
            CockpitSection::PcHost => Some("PC".to_string()),
            CockpitSection::Approvals if self.pending_approvals > 0 => {
                Some(self.pending_approvals.to_string())
            }
            CockpitSection::Diagnostics if self.diagnostics_errors > 0 => {
                Some(self.diagnostics_errors.to_string())
            }
            CockpitSection::Diagnostics if self.diagnostics_warnings > 0 => {
                Some(self.diagnostics_warnings.to_string())
            }
            CockpitSection::Git if self.dirty_files > 0 => Some(self.dirty_files.to_string()),
            CockpitSection::Tasks if self.running_tasks > 0 => Some(self.running_tasks.to_string()),
            CockpitSection::Chat if self.native_waiting => Some("…".to_string()),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DrawerItem {
    pub section: CockpitSection,
    pub title: &'static str,
    pub subtitle: &'static str,
}

pub fn default_drawer_items() -> Vec<DrawerItem> {
    vec![
        item(CockpitSection::Chat),
        item(CockpitSection::PcHost),
        item(CockpitSection::Files),
        item(CockpitSection::Snapshots),
        item(CockpitSection::Diagnostics),
        item(CockpitSection::Terminal),
        item(CockpitSection::Approvals),
        item(CockpitSection::Mcp),
        item(CockpitSection::Skills),
        item(CockpitSection::Git),
        item(CockpitSection::Tasks),
        item(CockpitSection::Settings),
    ]
}

fn item(section: CockpitSection) -> DrawerItem {
    DrawerItem {
        section,
        title: section.title(),
        subtitle: section.subtitle(),
    }
}

pub fn mobile_drawer(
    is_open: bool,
    active_section: CockpitSection,
    summary: MobileChromeSummary,
    on_select: EventHandler<CockpitSection>,
) -> Element {
    if !is_open {
        return rsx! {};
    }

    let items = default_drawer_items();

    rsx! {
        div {
            position: "absolute",
            left: "0",
            top: "0",
            bottom: "0",
            width: "86%",
            max_width: "420px",
            background_color: "#0b1018",
            color: "white",
            border_right: "1px solid #374151",
            padding: "18px",
            z_index: "10",
            display: "flex",
            flex_direction: "column",
            gap: "14px",
            overflow_y: "auto",
            box_shadow: "18px 0 40px rgba(0, 0, 0, 0.35)",

            div {
                display: "flex",
                flex_direction: "column",
                gap: "4px",
                div {
                    font_size: "22px",
                    font_weight: "bold",
                    "DeepSeek Mobile"
                }
                div {
                    color: "#9ca3af",
                    font_size: "13px",
                    "AI coding cockpit drawer"
                }
            }

            div {
                background_color: "#111827",
                border: "1px solid #374151",
                border_radius: "16px",
                padding: "12px",
                display: "flex",
                flex_direction: "column",
                gap: "8px",

                div {
                    color: "#9ca3af",
                    font_size: "12px",
                    "ACTIVE WORKSPACE"
                }
                div {
                    font_size: "16px",
                    font_weight: "bold",
                    white_space: "nowrap",
                    overflow: "hidden",
                    text_overflow: "ellipsis",
                    "{summary.active_project_title}"
                }
                div {
                    color: "#9ca3af",
                    font_size: "13px",
                    white_space: "nowrap",
                    overflow: "hidden",
                    text_overflow: "ellipsis",
                    "{summary.active_project_subtitle}"
                }
            }

            div {
                display: "flex",
                flex_direction: "column",
                gap: "10px",

                for item in items {
                    {
                        let badge = summary.badge_for(item.section);
                        rsx! {
                            button {
                                background_color: if item.section == active_section { "#1e3a8a" } else { "#111827" },
                                color: "white",
                                border: if item.section == active_section { "1px solid #3b82f6" } else { "1px solid #1f2937" },
                                border_radius: "14px",
                                padding: "12px",
                                display: "flex",
                                justify_content: "space-between",
                                align_items: "center",
                                gap: "12px",
                                text_align: "left",
                                onclick: move |_| on_select.call(item.section),

                                div {
                                    display: "flex",
                                    flex_direction: "column",
                                    gap: "3px",
                                    min_width: "0",
                                    div {
                                        font_size: "15px",
                                        font_weight: "bold",
                                        "{item.title}"
                                    }
                                    div {
                                        color: "#d1d5db",
                                        font_size: "12px",
                                        "{item.subtitle}"
                                    }
                                }

                                if let Some(badge) = badge {
                                    div {
                                        background_color: badge_background(item.section, &badge),
                                        color: "white",
                                        border_radius: "999px",
                                        padding: "4px 8px",
                                        font_size: "11px",
                                        font_weight: "bold",
                                        white_space: "nowrap",
                                        "{badge}"
                                    }
                                }
                            }
                        }
                    }
                }
            }

            div {
                background_color: "#111827",
                border: "1px solid #374151",
                border_radius: "16px",
                padding: "12px",

                div {
                    color: "#9ca3af",
                    font_size: "12px",
                    text_align: "center",
                    "DeepSeek Mobile v0.1 — Android preview"
                }
                div {
                    color: "#4b5563",
                    font_size: "11px",
                    text_align: "center",
                    margin_top: "3px",
                    "GitHub · DeepSeek API · PC Host"
                }
            }
        }
    }
}

fn badge_background(section: CockpitSection, badge: &str) -> &'static str {
    match section {
        CockpitSection::PcHost if badge.contains("ON") => "#047857",
        CockpitSection::PcHost if badge.contains("ERR") || badge.contains("OFF") => "#b91c1c",
        CockpitSection::PcHost => "#2563eb",
        CockpitSection::Approvals => "#ca8a04",
        CockpitSection::Diagnostics if badge.contains("ERR") => "#b91c1c",
        CockpitSection::Diagnostics => "#a16207",
        CockpitSection::Git => "#7c3aed",
        CockpitSection::Tasks => "#0891b2",
        CockpitSection::Chat => "#2563eb",
        _ => "#374151",
    }
}

// Bottom navigation bar for quick section switching
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NavItem {
    pub section: CockpitSection,
    pub label: &'static str,
    pub short: &'static str,
}

pub fn default_nav_items() -> Vec<NavItem> {
    vec![
        NavItem {
            section: CockpitSection::Chat,
            label: "Chat",
            short: "💬",
        },
        NavItem {
            section: CockpitSection::Skills,
            label: "Skills",
            short: "⚡",
        },
        NavItem {
            section: CockpitSection::Mcp,
            label: "MCP",
            short: "🔌",
        },
        NavItem {
            section: CockpitSection::PcHost,
            label: "PC",
            short: "🖥",
        },
        NavItem {
            section: CockpitSection::Files,
            label: "Files",
            short: "📁",
        },
        NavItem {
            section: CockpitSection::Terminal,
            label: "Term",
            short: ">",
        },
        NavItem {
            section: CockpitSection::Approvals,
            label: "Approve",
            short: "✓",
        },
        NavItem {
            section: CockpitSection::Git,
            label: "Git",
            short: "⬡",
        },
        NavItem {
            section: CockpitSection::Tasks,
            label: "Tasks",
            short: "⚙",
        },
    ]
}

pub fn bottom_nav_bar(
    active_section: CockpitSection,
    summary: MobileChromeSummary,
    on_select: EventHandler<CockpitSection>,
) -> Element {
    let items = default_nav_items();

    rsx! {
        div {
            background_color: "#0b1018",
            border_top: "1px solid #1f2937",
            padding: "6px 0 4px",
            display: "flex",
            justify_content: "flex-start",
            gap: "8px",
            overflow_x: "auto",

            for item in items {
                {
                    let badge = summary.compact_badge_for(item.section);
                    rsx! {
                        button {
                            position: "relative",
                            display: "flex",
                            flex_direction: "column",
                            align_items: "center",
                            gap: "2px",
                            min_width: "64px",
                            padding: "6px 10px",
                            background_color: if item.section == active_section { "#111827" } else { "transparent" },
                            border: if item.section == active_section { "1px solid #1d4ed8" } else { "1px solid transparent" },
                            border_radius: "14px",
                            color: if item.section == active_section { "#60a5fa" } else { "#9ca3af" },
                            font_size: "11px",
                            onclick: move |_| on_select.call(item.section),

                            if let Some(badge) = badge {
                                div {
                                    position: "absolute",
                                    top: "2px",
                                    right: "6px",
                                    background_color: badge_background(item.section, &badge),
                                    color: "white",
                                    border_radius: "999px",
                                    min_width: "16px",
                                    height: "16px",
                                    padding: "0 4px",
                                    font_size: "9px",
                                    line_height: "16px",
                                    font_weight: "bold",
                                    "{badge}"
                                }
                            }
                            div {
                                font_size: "18px",
                                "{item.short}"
                            }
                            div {
                                font_size: "10px",
                                "{item.label}"
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{CockpitSection, MobileChromeSummary};

    #[test]
    fn all_non_chat_sections_have_titles() {
        assert_eq!(CockpitSection::PcHost.title(), "PC Host");
        assert_eq!(CockpitSection::Files.title(), "Files");
        assert_eq!(CockpitSection::Snapshots.title(), "Snapshots");
        assert_eq!(CockpitSection::Diagnostics.title(), "Diagnostics");
        assert_eq!(CockpitSection::Terminal.title(), "Terminal");
        assert_eq!(CockpitSection::Approvals.title(), "Approvals");
        assert_eq!(CockpitSection::Git.title(), "Git & GitHub");
        assert_eq!(CockpitSection::Mcp.title(), "MCP");
        assert_eq!(CockpitSection::Skills.title(), "Skills");
        assert_eq!(CockpitSection::Tasks.title(), "Tasks");
        assert_eq!(CockpitSection::Settings.title(), "Settings");
    }

    #[test]
    fn drawer_items_include_all_sections() {
        let items = super::default_drawer_items();
        let titles: Vec<&str> = items.iter().map(|i| i.title).collect();
        assert!(titles.contains(&"Chat"));
        assert!(titles.contains(&"MCP"));
        assert!(titles.contains(&"Skills"));
        assert!(titles.contains(&"Tasks"));
        assert!(titles.contains(&"Settings"));
    }

    #[test]
    fn bottom_nav_includes_all_new_sections() {
        let items = super::default_nav_items();
        let labels: Vec<&str> = items.iter().map(|i| i.label).collect();
        assert!(labels.contains(&"MCP"));
        assert!(labels.contains(&"Skills"));
        assert!(labels.contains(&"Tasks"));
    }

    #[test]
    fn chrome_summary_badges_reflect_runtime_counts() {
        let summary = MobileChromeSummary {
            api_configured: true,
            pc_label: "PC ON".to_string(),
            pc_online: true,
            pending_approvals: 3,
            running_tasks: 2,
            diagnostics_errors: 1,
            dirty_files: 4,
            native_waiting: true,
            ..MobileChromeSummary::default()
        };

        assert_eq!(summary.api_chip_label(), "API OK");
        assert_eq!(
            summary.badge_for(CockpitSection::PcHost).as_deref(),
            Some("PC ON")
        );
        assert_eq!(
            summary.badge_for(CockpitSection::Approvals).as_deref(),
            Some("3")
        );
        assert_eq!(
            summary.badge_for(CockpitSection::Diagnostics).as_deref(),
            Some("ERR 1")
        );
        assert_eq!(summary.badge_for(CockpitSection::Git).as_deref(), Some("4"));
        assert_eq!(
            summary.badge_for(CockpitSection::Tasks).as_deref(),
            Some("RUN 2")
        );
        assert_eq!(
            summary.badge_for(CockpitSection::Chat).as_deref(),
            Some("WAIT")
        );
    }
}
