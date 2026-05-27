//! Responsive layout tokens (clamp-based for different phone widths).

pub const APP_FONT_STACK: &str =
    "system-ui, -apple-system, 'Segoe UI', Roboto, 'Noto Sans', 'Helvetica Neue', sans-serif";

#[derive(Clone, Debug)]
pub struct ScreenLayout {
    pub viewport_padding: &'static str,
    pub card_max_width: &'static str,
    pub title_font: &'static str,
    pub subtitle_font: &'static str,
    pub body_font: &'static str,
    pub caption_font: &'static str,
    pub button_padding: &'static str,
    pub header_title_font: &'static str,
    pub chat_input_min_height: &'static str,
    pub line_height: &'static str,
}

pub fn screen_layout() -> ScreenLayout {
    ScreenLayout {
        viewport_padding: "calc(env(safe-area-inset-top, 0px) + 2px) 6px calc(env(safe-area-inset-bottom, 0px) + 0px)",
        card_max_width: "100%",
        title_font: "clamp(1.2rem, 4vw, 1.65rem)",
        subtitle_font: "clamp(0.72rem, 2.6vw, 0.85rem)",
        body_font: "clamp(0.875rem, 2.8vw, 1rem)",
        caption_font: "clamp(0.65rem, 2.2vw, 0.75rem)",
        button_padding: "clamp(10px, 3vw, 14px)",
        header_title_font: "clamp(0.95rem, 3.5vw, 1.1rem)",
        chat_input_min_height: "clamp(48px, 12vw, 58px)",
        line_height: "1.45",
    }
}

pub fn stack_shell_style(layout: &ScreenLayout) -> String {
    format!(
        "display:flex;flex-direction:column;height:100dvh;min-height:100vh;width:100vw;max-width:100vw;overflow:hidden;padding:{};box-sizing:border-box;font-family:{};font-size:{};line-height:{};-webkit-font-smoothing:antialiased;color:#f9fafb;background:#05070c;",
        layout.viewport_padding, APP_FONT_STACK, layout.body_font, layout.line_height
    )
}

/// Toolbar + agent mode chips: stays above the scrolling timeline.
pub fn chat_sticky_chrome_style() -> &'static str {
    "flex-shrink:0;margin-bottom:clamp(2px,1vw,6px);"
}

/// Single card for workspace line + chat actions + mode chips.
pub fn chat_chrome_card_style() -> &'static str {
    "flex-shrink:0;background:#0f172a;border:1px solid #1e293b;border-radius:12px;padding:8px 10px;"
}

pub fn app_header_style() -> &'static str {
    "display:flex;align-items:center;justify-content:space-between;margin-bottom:clamp(4px,1.2vw,8px);gap:6px;flex-shrink:0;min-height:0;"
}

pub fn chrome_action_btn_style(primary: bool) -> String {
    let (bg, border, color) = if primary {
        ("#0b1220", "#1d4ed8", "#93c5fd")
    } else {
        ("#111827", "#334155", "#cbd5e1")
    };
    format!(
        "background:{bg};color:{color};border:1px solid {border};border-radius:999px;padding:4px 8px;font-size:clamp(0.62rem,2vw,0.7rem);font-weight:600;min-height:28px;line-height:1;white-space:nowrap;"
    )
}

pub fn mode_chip_style(border: &str, color: &str) -> String {
    format!(
        "flex:1;min-width:0;background:#0b1220;color:{color};border:1px solid {border};border-radius:999px;padding:5px 4px;font-size:clamp(0.62rem,2vw,0.72rem);font-weight:700;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;text-align:center;"
    )
}

/// Main scroll region (timeline / section panels).
pub fn main_scroll_panel_style() -> &'static str {
    "flex:1;min-height:0;background-color:#111827;padding:10px;border-radius:14px;overflow-y:auto;display:flex;flex-direction:column;gap:8px;-webkit-overflow-scrolling:touch;"
}

/// Compact workspace strip (non-chat sections).
pub fn workspace_strip_style(layout: &ScreenLayout) -> String {
    format!(
        "flex-shrink:0;background:#0f172a;border:1px solid #1e293b;border-radius:12px;padding:6px 10px;margin-bottom:clamp(4px,1vw,8px);display:flex;align-items:center;justify-content:space-between;gap:8px;min-height:0;font-size:{};",
        layout.subtitle_font
    )
}

pub fn workspace_strip_compact_style(layout: &ScreenLayout) -> String {
    format!("{}line-height:1.3;", workspace_strip_style(layout))
}

pub fn centered_card_style(layout: &ScreenLayout) -> String {
    format!(
        "width:100%;max-width:{};box-sizing:border-box;",
        layout.card_max_width
    )
}
