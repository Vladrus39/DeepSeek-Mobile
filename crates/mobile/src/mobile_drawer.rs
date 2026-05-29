use crate::locale::{pick, tr, AppLanguage, Tr};
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
    Health,
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
            CockpitSection::Health => "Health",
            CockpitSection::Settings => "Settings",
        }
    }

    pub fn localized_title(self, lang: AppLanguage) -> &'static str {
        match (lang, self) {
            (AppLanguage::Ru, CockpitSection::Chat) => "Чат",
            (AppLanguage::Ru, CockpitSection::PcHost) => "PC Host",
            (AppLanguage::Ru, CockpitSection::Files) => "Файлы",
            (AppLanguage::Ru, CockpitSection::Snapshots) => "Снимки",
            (AppLanguage::Ru, CockpitSection::Diagnostics) => "Диагностика",
            (AppLanguage::Ru, CockpitSection::Terminal) => "Терминал",
            (AppLanguage::Ru, CockpitSection::Approvals) => "Одобрения",
            (AppLanguage::Ru, CockpitSection::Mcp) => "MCP",
            (AppLanguage::Ru, CockpitSection::Skills) => "Навыки",
            (AppLanguage::Ru, CockpitSection::Git) => "Git и GitHub",
            (AppLanguage::Ru, CockpitSection::Tasks) => "Задачи",
            (AppLanguage::Ru, CockpitSection::Health) => "Состояние",
            (AppLanguage::Ru, CockpitSection::Settings) => "Настройки",
            (_, s) => s.title(),
        }
    }

    pub fn localized_subtitle(self, lang: AppLanguage) -> &'static str {
        match (lang, self) {
            (AppLanguage::Ru, CockpitSection::Chat) => "Диалог с ИИ и лента инструментов",
            (AppLanguage::Ru, CockpitSection::PcHost) => "Сопряжение, статус, рабочие области",
            (AppLanguage::Ru, CockpitSection::Files) => "Дерево проекта, файлы, diff",
            (AppLanguage::Ru, CockpitSection::Snapshots) => "Точки восстановления",
            (AppLanguage::Ru, CockpitSection::Diagnostics) => {
                "Ошибки и предупреждения после правок"
            }
            (AppLanguage::Ru, CockpitSection::Terminal) => "Вывод PC / Termux",
            (AppLanguage::Ru, CockpitSection::Approvals) => {
                "Вызовы инструментов, ждущие подтверждения"
            }
            (AppLanguage::Ru, CockpitSection::Mcp) => "MCP-серверы и конфиг",
            (AppLanguage::Ru, CockpitSection::Skills) => "Наборы навыков",
            (AppLanguage::Ru, CockpitSection::Git) => "Статус, коммиты, push, pull",
            (AppLanguage::Ru, CockpitSection::Tasks) => "Фоновые задачи и сборки",
            (AppLanguage::Ru, CockpitSection::Health) => "API, PC, Termux, MCP",
            (AppLanguage::Ru, CockpitSection::Settings) => "API, GitHub, Termux",
            (_, s) => s.subtitle(),
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
            CockpitSection::Health => "API, PC, Termux, MCP and bridge status",
            CockpitSection::Settings => "DeepSeek API, GitHub, disks, security",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MobileChromeSummary {
    pub api_configured: bool,
    pub pc_label: String,
    pub pc_online: bool,
    pub pc_files_sync_ready: bool,
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
            pc_files_sync_ready: false,
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
    pub fn api_chip_label(&self, lang: AppLanguage) -> &'static str {
        if self.api_configured {
            pick(lang, "API OK", "API OK")
        } else {
            pick(lang, "НЕТ API", "NO API")
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
        if self.pc_files_sync_ready {
            ("#052e2b", "#10b981", "#a7f3d0")
        } else if self.pc_online {
            ("#78350f", "#f59e0b", "#fde68a")
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

pub fn default_drawer_items(lang: AppLanguage) -> Vec<DrawerItem> {
    [
        CockpitSection::Chat,
        CockpitSection::PcHost,
        CockpitSection::Files,
        CockpitSection::Snapshots,
        CockpitSection::Diagnostics,
        CockpitSection::Terminal,
        CockpitSection::Approvals,
        CockpitSection::Mcp,
        CockpitSection::Skills,
        CockpitSection::Git,
        CockpitSection::Tasks,
        CockpitSection::Health,
        CockpitSection::Settings,
    ]
    .into_iter()
    .map(|section| DrawerItem {
        section,
        title: section.localized_title(lang),
        subtitle: section.localized_subtitle(lang),
    })
    .collect()
}

pub fn mobile_drawer(
    is_open: bool,
    active_section: CockpitSection,
    summary: MobileChromeSummary,
    lang: AppLanguage,
    on_select: EventHandler<CockpitSection>,
    on_close: EventHandler<()>,
) -> Element {
    if !is_open {
        return rsx! {};
    }

    let items = default_drawer_items(lang);
    let drawer_subtitle = pick(lang, "Панель кабины ИИ-кодинга", "AI coding cockpit drawer");
    let active_workspace_label = pick(lang, "АКТИВНЫЙ ПРОЕКТ", "ACTIVE WORKSPACE");
    let footer_version = pick(
        lang,
        "DeepSeek Mobile v0.1 — Android",
        "DeepSeek Mobile v0.1 — Android preview",
    );
    let footer_integrations = "GitHub · DeepSeek API · PC Host";

    rsx! {
        div {
            position: "absolute",
            left: "0",
            top: "0",
            right: "0",
            bottom: "0",
            background_color: "rgba(0, 0, 0, 0.42)",
            z_index: "9",
            onclick: move |_| on_close.call(()),
        }

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
            padding: "calc(18px + env(safe-area-inset-top, 0px)) 18px 18px",
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
                    "{tr(lang, Tr::AppTitle)}"
                }
                div {
                    color: "#9ca3af",
                    font_size: "13px",
                    "{drawer_subtitle}"
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
                    "{active_workspace_label}"
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
                    "{footer_version}"
                }
                div {
                    color: "#4b5563",
                    font_size: "11px",
                    text_align: "center",
                    margin_top: "3px",
                    "{footer_integrations}"
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

pub fn default_nav_items(lang: AppLanguage) -> Vec<NavItem> {
    vec![
        NavItem {
            section: CockpitSection::Chat,
            label: CockpitSection::Chat.localized_title(lang),
            short: "💬",
        },
        NavItem {
            section: CockpitSection::Skills,
            label: CockpitSection::Skills.localized_title(lang),
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
            label: CockpitSection::Files.localized_title(lang),
            short: "📁",
        },
        NavItem {
            section: CockpitSection::Approvals,
            label: CockpitSection::Approvals.localized_title(lang),
            short: "✓",
        },
        NavItem {
            section: CockpitSection::Health,
            label: CockpitSection::Health.localized_title(lang),
            short: "♥",
        },
    ]
}

pub fn bottom_nav_bar(
    active_section: CockpitSection,
    summary: MobileChromeSummary,
    lang: AppLanguage,
    on_select: EventHandler<CockpitSection>,
) -> Element {
    let items = default_nav_items(lang);

    rsx! {
        div {
            background_color: "#0b1018",
            border_top: "1px solid #1f2937",
            padding: "6px 4px 4px",
            display: "grid",
            grid_template_columns: "repeat(7, minmax(0, 1fr))",
            gap: "4px",
            overflow_x: "hidden",

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
                            min_width: "0",
                            padding: "6px 2px",
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
                                    right: "2px",
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
                                font_size: "9px",
                                max_width: "100%",
                                overflow: "hidden",
                                text_overflow: "ellipsis",
                                white_space: "nowrap",
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
        let items = super::default_drawer_items(crate::locale::AppLanguage::En);
        let titles: Vec<&str> = items.iter().map(|i| i.title).collect();
        assert!(titles.contains(&"Chat"));
        assert!(titles.contains(&"MCP"));
        assert!(titles.contains(&"Skills"));
        assert!(titles.contains(&"Tasks"));
        assert!(titles.contains(&"Settings"));
    }

    #[test]
    fn bottom_nav_includes_all_new_sections() {
        let items = super::default_nav_items(crate::locale::AppLanguage::En);
        let labels: Vec<&str> = items.iter().map(|i| i.label).collect();
        assert!(labels.contains(&"MCP"));
        assert!(labels.contains(&"Skills"));
        assert!(labels.contains(&"Approvals"));
        assert!(labels.contains(&"Health"));
        assert_eq!(items.len(), 7);
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

        assert_eq!(
            summary.api_chip_label(crate::locale::AppLanguage::En),
            "API OK"
        );
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
