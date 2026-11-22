//! ChxChx remote client (ADR 0017): the web app the node serves to the person's own devices.

mod actions;
mod code;
mod engine;
mod model;
mod net;
mod route;
mod store;
mod ui;

fn main() {
    dioxus::launch(ui::app);
}
