//! Compact system readiness indicators for the chat shell.

use crate::locale::{pick, AppLanguage};
use crate::mobile_drawer::CockpitSection;
use crate::runtime_health::RuntimeHealthSnapshot;
use dioxus::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReadinessSegment {
    Api,
    Phone,
    PcPing,
    PcFiles,
    Bridge,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReadinessLevel {
    Ok,
    Warn,
    Bad,
    Idle,
}

#[derive(Clone, Debug)]
pub struct ReadinessItem {
    pub segment: ReadinessSegment,
    pub level: ReadinessLevel,
    pub short_label: String,
    pub detail: String,
    pub target_section: CockpitSection,
}

impl RuntimeHealthSnapshot {
    pub fn readiness_items(&self, lang: AppLanguage) -> Vec<ReadinessItem> {
        let phone_level = if self.full_agent_on_phone_ready {
            ReadinessLevel::Ok
        } else if self.termux_valid || self.api_configured {
            ReadinessLevel::Warn
        } else {
            ReadinessLevel::Bad
        };

        let pc_ping_level = if self.pc_online {
            ReadinessLevel::Ok
        } else if self.pc_status_label.contains("Waiting")
            || self.pc_status_label.contains("Ожидание")
        {
            ReadinessLevel::Warn
        } else if self.pc_workspace_active
            || self.pc_status_label.contains("Not configured")
            || self.pc_status_label.contains("Не настроен")
        {
            ReadinessLevel::Idle
        } else {
            ReadinessLevel::Bad
        };

        let pc_files_level = if self.pc_files_sync_ready {
            ReadinessLevel::Ok
        } else if self.pc_online {
            ReadinessLevel::Warn
        } else {
            ReadinessLevel::Idle
        };

        let bridge_level = if self.native_pending {
            ReadinessLevel::Warn
        } else if self.native_last_error.is_some() {
            ReadinessLevel::Bad
        } else {
            ReadinessLevel::Ok
        };

        vec![
            ReadinessItem {
                segment: ReadinessSegment::Api,
                level: if self.api_configured {
                    ReadinessLevel::Ok
                } else {
                    ReadinessLevel::Bad
                },
                short_label: pick(lang, "API", "API").to_string(),
                detail: if self.api_configured {
                    pick(lang, "Ключ задан", "Key set").to_string()
                } else {
                    pick(lang, "Нет ключа", "Missing key").to_string()
                },
                target_section: CockpitSection::Settings,
            },
            ReadinessItem {
                segment: ReadinessSegment::Phone,
                level: phone_level,
                short_label: pick(lang, "Телефон", "Phone").to_string(),
                detail: if self.full_agent_on_phone_ready {
                    pick(lang, "Termux готов", "Termux ready").to_string()
                } else if self.termux_valid {
                    pick(lang, "Нужен API", "Needs API key").to_string()
                } else {
                    pick(lang, "Termux", "Termux").to_string()
                },
                target_section: CockpitSection::Settings,
            },
            ReadinessItem {
                segment: ReadinessSegment::PcPing,
                level: pc_ping_level,
                short_label: pick(lang, "PC", "PC").to_string(),
                detail: self.pc_status_label.clone(),
                target_section: CockpitSection::PcHost,
            },
            ReadinessItem {
                segment: ReadinessSegment::PcFiles,
                level: pc_files_level,
                short_label: pick(lang, "Файлы PC", "PC files").to_string(),
                detail: if self.pc_files_sync_ready {
                    pick(lang, "Синхронизация OK", "Sync OK").to_string()
                } else if self.pc_online {
                    pick(lang, "Нужен pairing", "Needs pairing").to_string()
                } else {
                    pick(lang, "Не используется", "Not used").to_string()
                },
                target_section: CockpitSection::Files,
            },
            ReadinessItem {
                segment: ReadinessSegment::Bridge,
                level: bridge_level,
                short_label: pick(lang, "Мост", "Bridge").to_string(),
                detail: if self.native_pending {
                    pick(lang, "WAIT", "WAIT").to_string()
                } else if self.native_last_error.is_some() {
                    pick(lang, "Ошибка", "Error").to_string()
                } else {
                    pick(lang, "OK", "OK").to_string()
                },
                target_section: CockpitSection::Health,
            },
        ]
    }
}

fn level_colors(level: ReadinessLevel) -> (&'static str, &'static str) {
    match level {
        ReadinessLevel::Ok => ("#064e3b", "#10b981"),
        ReadinessLevel::Warn => ("#78350f", "#f59e0b"),
        ReadinessLevel::Bad => ("#7f1d1d", "#ef4444"),
        ReadinessLevel::Idle => ("#1f2937", "#6b7280"),
    }
}

pub fn readiness_strip(
    lang: AppLanguage,
    snapshot: &RuntimeHealthSnapshot,
    on_select_section: EventHandler<CockpitSection>,
) -> Element {
    let items = snapshot.readiness_items(lang);
    rsx! {
        div {
            style: "display:flex;flex-wrap:wrap;gap:6px;padding:6px 8px;background:#0b1220;border:1px solid #1f2937;border-radius:12px;",
            for item in items {
                {
                    let (bg, border) = level_colors(item.level);
                    let border_style = format!("1px solid {border}");
                    let title = format!("{} — {}", item.short_label, item.detail);
                    let section = item.target_section;
                    let label = format!("{} · {}", item.short_label, item.detail);
                    rsx! {
                        button {
                            key: "{item.segment:?}",
                            title: "{title}",
                            style: "background:{bg};border:{border_style};border-radius:999px;padding:4px 8px;font-size:10px;font-weight:700;color:#f9fafb;max-width:100%;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;",
                            onclick: move |_| on_select_section.call(section),
                            "{label}"
                        }
                    }
                }
            }
        }
    }
}
