//! Responsive layout tokens (clamp-based for different phone widths).

#[derive(Clone, Debug)]
pub struct ScreenLayout {
    pub viewport_padding: &'static str,
    pub card_max_width: &'static str,
    pub title_font: &'static str,
    pub subtitle_font: &'static str,
    pub body_font: &'static str,
    pub button_padding: &'static str,
    pub header_title_font: &'static str,
    pub chat_input_min_height: &'static str,
}

pub fn screen_layout() -> ScreenLayout {
    ScreenLayout {
        viewport_padding: "clamp(10px, 3vw, 20px)",
        card_max_width: "min(100%, 28rem)",
        title_font: "clamp(1.35rem, 4.5vw, 1.75rem)",
        subtitle_font: "clamp(0.75rem, 2.8vw, 0.875rem)",
        body_font: "clamp(0.8125rem, 2.6vw, 0.9375rem)",
        button_padding: "clamp(12px, 3.5vw, 16px)",
        header_title_font: "clamp(1rem, 3.8vw, 1.125rem)",
        chat_input_min_height: "clamp(44px, 12vw, 56px)",
    }
}

pub fn stack_shell_style(layout: &ScreenLayout) -> String {
    format!(
        "display:flex;flex-direction:column;height:100vh;width:100%;max-width:100vw;overflow:hidden;padding:{};box-sizing:border-box;",
        layout.viewport_padding
    )
}

pub fn centered_card_style(layout: &ScreenLayout) -> String {
    format!(
        "width:100%;max-width:{};box-sizing:border-box;",
        layout.card_max_width
    )
}
