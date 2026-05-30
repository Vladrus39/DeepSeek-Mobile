//! Keep the chat timeline scrolled to the latest message (WebView fallback).

pub const CHAT_SCROLL_PANEL_ID: &str = "deepseek-chat-scroll";

const STICKY_THRESHOLD_PX: u32 = 160;

/// Scroll to bottom only when the user is already near the bottom, or when `force` is true
/// (new user message, thread switch, returning to chat).
pub fn scroll_chat_to_bottom_sticky(force: bool) {
    let script = format!(
        r#"
        (function() {{
            const el = document.getElementById("{CHAT_SCROLL_PANEL_ID}");
            if (!el) return;
            const force = {force};
            const threshold = {STICKY_THRESHOLD_PX};
            const nearBottom = el.scrollHeight - el.scrollTop - el.clientHeight <= threshold;
            if (!force && !nearBottom) return;
            const scroll = () => {{ el.scrollTop = el.scrollHeight; }};
            scroll();
            requestAnimationFrame(scroll);
        }})();
        "#
    );
    let _ = dioxus::document::eval(&script);
}

/// Always scroll to the latest message (e.g. after the user sends a message).
pub fn scroll_chat_to_bottom_force() {
    scroll_chat_to_bottom_sticky(true);
}
