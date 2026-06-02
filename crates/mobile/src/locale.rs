//! UI strings (Russian / English) and persistence.

use crate::mobile_runtime_config::default_data_dir;
use serde::{Deserialize, Serialize};
use std::fs;
use std::sync::OnceLock;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AppLanguage {
    #[default]
    Ru,
    En,
}

impl AppLanguage {
    pub fn toggle(self) -> Self {
        match self {
            AppLanguage::Ru => AppLanguage::En,
            AppLanguage::En => AppLanguage::Ru,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            AppLanguage::Ru => "RU",
            AppLanguage::En => "EN",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Tr {
    SetupTitle,
    SetupSubtitle,
    SetupApiLabel,
    SetupApiPlaceholder,
    SetupTermuxLabel,
    SetupTermuxHint,
    SetupTermuxSteps,
    SetupInstallTermux,
    SetupOpenTermux,
    SetupProbeTermux,
    SetupContinue,
    SetupSandboxOnly,
    SetupErrApiPrefix,
    CheckApi,
    CheckAgent,
    CheckTermux,
    SettingsTitle,
    SettingsLanguage,
    HealthTitle,
    HealthSubtitle,
    Send,
    ChatPlaceholder,
    Thinking,
    AppTitle,
    DrawerMenu,
    ContinueSandboxSaved,
}

pub fn tr(lang: AppLanguage, key: Tr) -> &'static str {
    match (lang, key) {
        (AppLanguage::Ru, Tr::SetupTitle) => "Настройка",
        (AppLanguage::En, Tr::SetupTitle) => "Setup",
        (AppLanguage::Ru, Tr::SetupSubtitle) => {
            "Один шаг: ключ API и путь Termux. Установите Termux из F-Droid, если ещё нет — отдельная кнопка не нужна."
        }
        (AppLanguage::En, Tr::SetupSubtitle) => {
            "One step: API key and Termux path. Install Termux from F-Droid if needed — no extra install button."
        }
        (AppLanguage::Ru, Tr::SetupApiLabel) => "Ключ DeepSeek API",
        (AppLanguage::En, Tr::SetupApiLabel) => "DeepSeek API key",
        (AppLanguage::Ru, Tr::SetupApiPlaceholder) => "sk-...",
        (AppLanguage::En, Tr::SetupApiPlaceholder) => "sk-...",
        (AppLanguage::Ru, Tr::SetupTermuxLabel) => "Путь проекта в Termux",
        (AppLanguage::En, Tr::SetupTermuxLabel) => "Termux project path",
        (AppLanguage::Ru, Tr::SetupTermuxHint) => {
            "Мы поможем настроить автоматически. Нажмите «Проверить RUN_COMMAND», затем используйте кнопки ниже."
        }
        (AppLanguage::En, Tr::SetupTermuxHint) => {
            "We'll help configure as much as possible automatically. Tap «Test RUN_COMMAND», then use the buttons below."
        }
        (AppLanguage::Ru, Tr::SetupTermuxSteps) => {
            "1) Установите/откройте Termux 2) Нажмите «Проверить RUN_COMMAND» (даст разрешение) 3) «Auto-config» + «Seed workspace»"
        }
        (AppLanguage::En, Tr::SetupTermuxSteps) => {
            "1) Install/open Termux 2) Tap «Test RUN_COMMAND» (grants permission) 3) Use «Auto-config» + «Seed workspace»"
        }
        (AppLanguage::Ru, Tr::SetupInstallTermux) => "Установить Termux",
        (AppLanguage::En, Tr::SetupInstallTermux) => "Install Termux",
        (AppLanguage::Ru, Tr::SetupOpenTermux) => "Открыть Termux",
        (AppLanguage::En, Tr::SetupOpenTermux) => "Open Termux",
        (AppLanguage::Ru, Tr::SetupProbeTermux) => "Дать разрешение и авто-настроить Termux",
        (AppLanguage::En, Tr::SetupProbeTermux) => "Grant permission & auto-setup Termux",
        (AppLanguage::Ru, Tr::SetupContinue) => "Продолжить",
        (AppLanguage::En, Tr::SetupContinue) => "Continue",
        (AppLanguage::Ru, Tr::SetupSandboxOnly) => "Только песочница (без Termux)",
        (AppLanguage::En, Tr::SetupSandboxOnly) => "Sandbox only (no Termux)",
        (AppLanguage::Ru, Tr::SetupErrApiPrefix) => "Ключ должен начинаться с sk-.",
        (AppLanguage::En, Tr::SetupErrApiPrefix) => "Key must start with sk-.",
        (AppLanguage::Ru, Tr::CheckApi) => "Ключ API",
        (AppLanguage::En, Tr::CheckApi) => "API key",
        (AppLanguage::Ru, Tr::CheckAgent) => "Режим Agent",
        (AppLanguage::En, Tr::CheckAgent) => "Agent mode",
        (AppLanguage::Ru, Tr::CheckTermux) => "Путь Termux",
        (AppLanguage::En, Tr::CheckTermux) => "Termux path",
        (AppLanguage::Ru, Tr::SettingsTitle) => "Настройки",
        (AppLanguage::En, Tr::SettingsTitle) => "Settings",
        (AppLanguage::Ru, Tr::SettingsLanguage) => "Язык интерфейса",
        (AppLanguage::En, Tr::SettingsLanguage) => "Interface language",
        (AppLanguage::Ru, Tr::HealthTitle) => "Состояние",
        (AppLanguage::En, Tr::HealthTitle) => "Runtime health",
        (AppLanguage::Ru, Tr::HealthSubtitle) => {
            "Полный агент на телефоне: Termux. PC Host — опционально."
        }
        (AppLanguage::En, Tr::HealthSubtitle) => {
            "Full agent on phone uses Termux. PC Host is optional."
        }
        (AppLanguage::Ru, Tr::Send) => "Отправить",
        (AppLanguage::En, Tr::Send) => "Send",
        (AppLanguage::Ru, Tr::ChatPlaceholder) => "Сообщение DeepSeek…",
        (AppLanguage::En, Tr::ChatPlaceholder) => "Message DeepSeek…",
        (AppLanguage::Ru, Tr::Thinking) => "Думаю с DeepSeek…",
        (AppLanguage::En, Tr::Thinking) => "Thinking with DeepSeek…",
        (AppLanguage::Ru, Tr::AppTitle) => "DeepSeek Mobile",
        (AppLanguage::En, Tr::AppTitle) => "DeepSeek Mobile",
        (AppLanguage::Ru, Tr::DrawerMenu) => "Меню",
        (AppLanguage::En, Tr::DrawerMenu) => "Menu",
        (AppLanguage::Ru, Tr::ContinueSandboxSaved) => "Режим песочницы — без shell в Termux",
        (AppLanguage::En, Tr::ContinueSandboxSaved) => "Sandbox mode — no Termux shell",
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct LocaleFile {
    language: AppLanguage,
}

static DEFAULT_LANG: OnceLock<AppLanguage> = OnceLock::new();

fn locale_path() -> std::path::PathBuf {
    default_data_dir().join("ui_locale.json")
}

pub fn load_ui_language() -> AppLanguage {
    if let Some(lang) = DEFAULT_LANG.get() {
        return *lang;
    }
    let lang = if locale_path().exists() {
        fs::read_to_string(locale_path())
            .ok()
            .and_then(|raw| serde_json::from_str::<LocaleFile>(&raw).ok())
            .map(|file| file.language)
            .unwrap_or_default()
    } else {
        AppLanguage::Ru
    };
    let _ = DEFAULT_LANG.set(lang);
    lang
}

/// Russian when `lang` is Ru, otherwise English.
pub fn pick(lang: AppLanguage, ru: &'static str, en: &'static str) -> &'static str {
    match lang {
        AppLanguage::Ru => ru,
        AppLanguage::En => en,
    }
}

use crate::agent_timeline::{MobileTimelineItemKind, MobileTimelineItemStatus};

pub fn timeline_kind_label(lang: AppLanguage, kind: &MobileTimelineItemKind) -> &'static str {
    match (lang, kind) {
        (AppLanguage::Ru, MobileTimelineItemKind::UserMessage) => "ВЫ",
        (AppLanguage::Ru, MobileTimelineItemKind::AssistantMessage) => "ИИ",
        (AppLanguage::Ru, MobileTimelineItemKind::Attachment) => "ФАЙЛ",
        (AppLanguage::Ru, MobileTimelineItemKind::NativeCommand) => "СИСТЕМА",
        (AppLanguage::Ru, MobileTimelineItemKind::ToolCall) => "ИНСТР",
        (AppLanguage::Ru, MobileTimelineItemKind::Approval) => "ОДОБР",
        (AppLanguage::Ru, MobileTimelineItemKind::Status) => "СТАТУС",
        (AppLanguage::Ru, MobileTimelineItemKind::Error) => "ОШИБКА",
        (AppLanguage::En, MobileTimelineItemKind::UserMessage) => "YOU",
        (AppLanguage::En, MobileTimelineItemKind::AssistantMessage) => "AI",
        (AppLanguage::En, MobileTimelineItemKind::Attachment) => "FILE",
        (AppLanguage::En, MobileTimelineItemKind::NativeCommand) => "NATIVE",
        (AppLanguage::En, MobileTimelineItemKind::ToolCall) => "TOOL",
        (AppLanguage::En, MobileTimelineItemKind::Approval) => "APPROVAL",
        (AppLanguage::En, MobileTimelineItemKind::Status) => "STATUS",
        (AppLanguage::En, MobileTimelineItemKind::Error) => "ERROR",
    }
}

pub fn timeline_status_label(lang: AppLanguage, status: &MobileTimelineItemStatus) -> &'static str {
    match (lang, status) {
        (AppLanguage::Ru, MobileTimelineItemStatus::Pending) => "ожидание",
        (AppLanguage::Ru, MobileTimelineItemStatus::Running) => "выполняется",
        (AppLanguage::Ru, MobileTimelineItemStatus::Done) => "готово",
        (AppLanguage::Ru, MobileTimelineItemStatus::Failed) => "ошибка",
        (AppLanguage::Ru, MobileTimelineItemStatus::WaitingForApproval) => "одобрение",
        (AppLanguage::En, MobileTimelineItemStatus::Pending) => "pending",
        (AppLanguage::En, MobileTimelineItemStatus::Running) => "running",
        (AppLanguage::En, MobileTimelineItemStatus::Done) => "done",
        (AppLanguage::En, MobileTimelineItemStatus::Failed) => "failed",
        (AppLanguage::En, MobileTimelineItemStatus::WaitingForApproval) => "approval",
    }
}

pub fn save_ui_language(lang: AppLanguage) -> Result<(), String> {
    let path = locale_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let file = LocaleFile { language: lang };
    fs::write(
        path,
        serde_json::to_string_pretty(&file).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    let _ = DEFAULT_LANG.set(lang);
    Ok(())
}
