use dioxus::prelude::*;

fn main() {
    dioxus_mobile::launch(app);
}

fn app() -> Element {
    rsx! {
        div {
            background_color: "#111111",
            color: "white",
            height: "100vh",
            display: "flex",
            flex_direction: "column",
            padding: "20px",
            
            h1 { "DeepSeek Mobile" }
            p { "Android Coding Agent (in development)" }
            
            // Chat placeholder
            div { "Chat interface will be here..." }
        }
    }
}
