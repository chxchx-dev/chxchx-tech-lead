use dioxus::prelude::*;

fn main() {
    launch(app);
}

fn app() -> Element {
    rsx! { main { h1 { "ChxChx" } } }
}
