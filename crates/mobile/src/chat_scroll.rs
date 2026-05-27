//! Keep the chat timeline scrolled to the latest message (WebView fallback).

pub const CHAT_SCROLL_PANEL_ID: &str = "deepseek-chat-scroll";

/// Scroll the main chat panel to the bottom after restore, thread switch, or new messages.
pub fn scroll_chat_to_bottom() {
    let script = format!(
        r#"
        (function() {{
            const el = document.getElementById("{CHAT_SCROLL_PANEL_ID}");
            if (!el) return;
            const scroll = () => {{ el.scrollTop = el.scrollHeight; }};
            scroll();
            requestAnimationFrame(scroll);
        }})();
        "#
    );
    let _ = dioxus::document::eval(&script);
}
