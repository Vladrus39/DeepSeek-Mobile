use dioxus::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DrawerItem {
    pub id: &'static str,
    pub title: &'static str,
    pub subtitle: &'static str,
    pub badge: Option<&'static str>,
}

pub fn default_drawer_items() -> Vec<DrawerItem> {
    vec![
        DrawerItem {
            id: "pc-host",
            title: "PC Host",
            subtitle: "Pairing, online status, workspaces",
            badge: Some("SETUP"),
        },
        DrawerItem {
            id: "files",
            title: "Files",
            subtitle: "Project tree, open files, diffs",
            badge: None,
        },
        DrawerItem {
            id: "terminal",
            title: "Terminal",
            subtitle: "PC / Termux command output",
            badge: None,
        },
        DrawerItem {
            id: "approvals",
            title: "Approvals",
            subtitle: "Tool calls waiting for confirmation",
            badge: Some("0"),
        },
        DrawerItem {
            id: "git",
            title: "Git & GitHub",
            subtitle: "Status, commits, push, pull, PRs",
            badge: None,
        },
        DrawerItem {
            id: "settings",
            title: "Settings",
            subtitle: "DeepSeek API, GitHub, disks, security",
            badge: None,
        },
    ]
}

pub fn mobile_drawer(is_open: bool) -> Element {
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
                    div {
                        background_color: "#111827",
                        border: "1px solid #1f2937",
                        border_radius: "14px",
                        padding: "12px",
                        display: "flex",
                        justify_content: "space-between",
                        align_items: "center",
                        gap: "12px",

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
                                color: "#9ca3af",
                                font_size: "12px",
                                "{item.subtitle}"
                            }
                        }

                        if let Some(badge) = item.badge {
                            div {
                                background_color: "#2563eb",
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

#[cfg(test)]
mod tests {
    use super::default_drawer_items;

    #[test]
    fn drawer_contains_core_cockpit_sections() {
        let items = default_drawer_items();
        let ids: Vec<&str> = items.iter().map(|item| item.id).collect();
        assert!(ids.contains(&"pc-host"));
        assert!(ids.contains(&"files"));
        assert!(ids.contains(&"terminal"));
        assert!(ids.contains(&"approvals"));
        assert!(ids.contains(&"git"));
        assert!(ids.contains(&"settings"));
    }
}
