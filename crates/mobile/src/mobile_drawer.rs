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
            CockpitSection::Git => "Status, commits, push, pull, PRs",
            CockpitSection::Tasks => "Background tasks, build jobs, test runs",
            CockpitSection::Settings => "DeepSeek API, GitHub, disks, security",
        }
    }

    pub fn badge(self) -> Option<&'static str> {
        match self {
            CockpitSection::PcHost => Some("SETUP"),
            CockpitSection::Approvals => Some("0"),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DrawerItem {
    pub section: CockpitSection,
    pub title: &'static str,
    pub subtitle: &'static str,
    pub badge: Option<&'static str>,
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
        badge: section.badge(),
    }
}

pub fn mobile_drawer(
    is_open: bool,
    active_section: CockpitSection,
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
            width: "82%",
            max_width: "420px",
            background_color: "#0b1018",
            color: "white",
            border_right: "1px solid #374151",
            padding: "18px",
            z_index: "10",
            display: "flex",
            flex_direction: "column",
            gap: "14px",

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
                    "ACTIVE PROJECT"
                }
                div {
                    font_size: "16px",
                    font_weight: "bold",
                    "No project selected"
                }
                div {
                    color: "#9ca3af",
                    font_size: "13px",
                    "Connect GitHub, local files or PC workspace"
                }
            }

            div {
                display: "flex",
                flex_direction: "column",
                gap: "10px",

                for item in items {
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

                        if let Some(badge) = item.badge {
                            div {
                                background_color: if item.section == CockpitSection::Approvals { "#ca8a04" } else { "#2563eb" },
                                color: "white",
                                border_radius: "999px",
                                padding: "4px 8px",
                                font_size: "11px",
                                font_weight: "bold",
                                "{badge}"
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
                    "GitHub · DeepSeek API · Y-Lit"
                }
            }
        }
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
        NavItem { section: CockpitSection::Chat, label: "Chat", short: "💬" },
        NavItem { section: CockpitSection::PcHost, label: "PC", short: "🖥" },
        NavItem { section: CockpitSection::Files, label: "Files", short: "📁" },
        NavItem { section: CockpitSection::Terminal, label: "Term", short: ">" },
        NavItem { section: CockpitSection::Approvals, label: "Approve", short: "✓" },
        NavItem { section: CockpitSection::Git, label: "Git", short: "⬡" },
        NavItem { section: CockpitSection::Tasks, label: "Tasks", short: "⚙" },
    ]
}

pub fn bottom_nav_bar(
    active_section: CockpitSection,
    on_select: EventHandler<CockpitSection>,
) -> Element {
    let items = default_nav_items();

    rsx! {
        div {
            background_color: "#0b1018",
            border_top: "1px solid #1f2937",
            padding: "4px 0",
            display: "flex",
            justify_content: "space_around",

            for item in items {
                button {
                    display: "flex",
                    flex_direction: "column",
                    align_items: "center",
                    gap: "2px",
                    padding: "6px 10px",
                    background_color: "transparent",
                    border: "none",
                    color: if item.section == active_section { "#3b82f6" } else { "#9ca3af" },
                    font_size: "11px",
                    onclick: move |_| on_select.call(item.section),

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

#[cfg(test)]
mod tests {
    use super::CockpitSection;

    #[test]
    fn all_non_chat_sections_have_titles() {
        assert_eq!(CockpitSection::PcHost.title(), "PC Host");
        assert_eq!(CockpitSection::Files.title(), "Files");
        assert_eq!(CockpitSection::Snapshots.title(), "Snapshots");
        assert_eq!(CockpitSection::Diagnostics.title(), "Diagnostics");
        assert_eq!(CockpitSection::Terminal.title(), "Terminal");
        assert_eq!(CockpitSection::Approvals.title(), "Approvals");
        assert_eq!(CockpitSection::Git.title(), "Git & GitHub");
        assert_eq!(CockpitSection::Tasks.title(), "Tasks");
        assert_eq!(CockpitSection::Settings.title(), "Settings");
    }

    #[test]
    fn drawer_items_include_all_sections() {
        let items = super::default_drawer_items();
        let titles: Vec<&str> = items.iter().map(|i| i.title).collect();
        assert!(titles.contains(&"Chat"));
        assert!(titles.contains(&"Tasks"));
        assert!(titles.contains(&"Settings"));
    }

    #[test]
    fn bottom_nav_includes_tasks() {
        let items = super::default_nav_items();
        let labels: Vec<&str> = items.iter().map(|i| i.label).collect();
        assert!(labels.contains(&"Tasks"));
    }
}
