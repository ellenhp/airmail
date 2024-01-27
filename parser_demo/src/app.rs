use yew::prelude::*;

use crate::text_input::TextInput;

pub enum Msg {
    SetPassword(String),
}

#[derive(Debug, Default)]
pub struct App {
    password: String,
}

impl App {
    fn readout_top_row_text(&self) -> String {
        let query = airmail_parser::query::Query::parse(&self.password.to_lowercase());
        let scenarios = query.scenarios();
        let scenario = scenarios.first().cloned();
        scenario.map_or_else(
            || "Unable to parse input".to_string(),
            |scenario| {
                let mut text = String::new();
                for component in &scenario.as_vec() {
                    text.push_str(&format!(
                        "{}: \"{}\"\n",
                        component.debug_name(),
                        component.text()
                    ));
                }
                text
            },
        )
    }
}

impl Component for App {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self::default()
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::SetPassword(next_password) => self.password = next_password,
        };
        true
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let on_change = ctx.link().callback(Msg::SetPassword);
        html! {
            <main>
                <div class="entry">
                    <div>
                        {"Enter a search query below:"}
                    </div>
                    <div>
                        <TextInput {on_change} value={self.password.clone()} />
                    </div>
                </div>
                <div class="readout">
                    {self.readout_top_row_text()}
                </div>
            </main>
        }
    }
}
