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
            padding: "20px",
            "DeepSeek Mobile - Core Connected"
            // Тут будет полный чат с streaming
        }
    }
}