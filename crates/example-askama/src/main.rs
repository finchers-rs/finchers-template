#[macro_use]
extern crate askama;
#[macro_use]
extern crate finchers;
extern crate finchers_template;
#[macro_use]
extern crate log;
extern crate pretty_env_logger;

use finchers::prelude::*;

use askama::Template;
use finchers_template::askama::TemplateExt;

#[derive(Debug, Template)]
#[template(path = "index.html")]
struct UserInfo {
    name: String,
}

fn main() {
    std::env::set_var("RUST_LOG", "example_askama=info");
    pretty_env_logger::init();

    let endpoint = path!(@get /).map(|| {
        UserInfo {
            name: "Alice".into(),
        }.into_rendered()
    });

    info!("Listening on http://127.0.0.1:4000");
    finchers::launch(endpoint).start("127.0.0.1:4000");
}
