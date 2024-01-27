#![recursion_limit = "256"]

mod app;
mod text_input;

use app::App;
use log::Level;

fn main() {
    console_log::init_with_level(Level::Trace).unwrap();
    yew::Renderer::<App>::new().render();
}
