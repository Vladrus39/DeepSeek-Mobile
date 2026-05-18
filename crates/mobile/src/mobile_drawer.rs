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
        }
    }
}


/// Bottom navigation bar component — 5 quick-access tabs at screen bottom.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BottomNavTab {
    pub section: CockpitSection,
    pub icon: &'static str,
    pub label: &'static str,
}

pub fn default_bottom_nav_tabs() -> Vec<BottomNavTab> {
    vec![
        BottomNavTab { section: CockpitSection::Chat, icon: "\u{1f4ac}", label: "Chat" },
        BottomNavTab { section: CockpitSection::Files, icon: "\u{1f4c1}", label: "Files" },
        BottomNavTab { section: CockpitSection::Terminal, icon: "\u{2328}", label: "Terminal" },
        BottomNavTab { section: CockpitSection::Git, icon: "\u{1f500}", label: "Git" },
        BottomNavTab { section: CockpitSection::Settings, icon: "\u{2699}", label: "Settings" },
    ]
}

pub fn bottom_nav_bar(
    active_section: CockpitSection,
    approval_count: usize,
    on_select: EventHandler<CockpitSection>,
) -> Element {
    let tabs = default_bottom_nav_tabs();

    rsx! {
        div {
            display: "flex",
            justify_content: "space-around",
            align_items: "center",
            background_color: "#0b1018",
            border_top: "1px solid #1f2937",
            padding: "6px 0",
            padding_bottom: "10px",
            position: "relative",
            z_index: "5",

            for tab in tabs {
                button {
                    key: "{tab.section.title()}",
                    position: "relative",
                    display: "flex",
                    flex_direction: "column",
                    align_items: "center",
                    gap: "2px",
                    background_color: "transparent",
                    border: "none",
                    color: if tab.section == active_section { "#3b82f6" } else { "#6b7280" },
                    font_size: if tab.section == active_section { "20px" } else { "18px" },
                    padding: "4px 12px",
                    onclick: move |_| on_select.call(tab.section),

                    div { "{tab.icon}" }
                    div {
                        font_size: "10px",
                        font_weight: if tab.section == active_section { "700" } else { "400" },
                        "{tab.label}"
                    }
                    if tab.section == CockpitSection::Chat && approval_count > 0 {
                        div {
                            position: "absolute",
                            top: "2px",
                            right: "2px",
                            background_color: "#ca8a04",
                            color: "white",
                            border_radius: "999px",
                            padding: "2px 6px",
                            font_size: "9px",
                            font_weight: "bold",
                            line_height: "1",
                            "{approval_count}"
                        }
                    }
                }
            }
        }
    }
}


#[cfg(test)]
mod tests {
    use super::{default_drawer_items, CockpitSection};

    #[test]
    fn drawer_contains_core_cockpit_sections() {
        let items = default_drawer_items();
        let sections: Vec<CockpitSection> = items.iter().map(|item| item.section).collect();
        assert!(sections.contains(&CockpitSection::Chat));
        assert!(sections.contains(&CockpitSection::PcHost));
        assert!(sections.contains(&CockpitSection::Files));
        assert!(sections.contains(&CockpitSection::Snapshots));
        assert!(sections.contains(&CockpitSection::Diagnostics));
        assert!(sections.contains(&CockpitSection::Terminal));
        assert!(sections.contains(&CockpitSection::Approvals));
        assert!(sections.contains(&CockpitSection::Git));
        assert!(sections.contains(&CockpitSection::Settings));
    }

    #[test]
    fn section_titles_are_stable() {
        assert_eq!(CockpitSection::Chat.title(), "Chat");
        assert_eq!(CockpitSection::PcHost.title(), "PC Host");
        assert_eq!(CockpitSection::Snapshots.title(), "Snapshots");
        assert_eq!(CockpitSection::Diagnostics.title(), "Diagnostics");
        assert_eq!(CockpitSection::Git.title(), "Git & GitHub");
    }
}
