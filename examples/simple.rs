#[macro_use]
extern crate finchers;
extern crate finchers_template_tera;
#[macro_use]
extern crate log;
extern crate pretty_env_logger;
#[macro_use]
extern crate serde;
extern crate tera;

use finchers::prelude::*;
use finchers_template_tera::Renderer;

use std::sync::Arc;
use tera::Tera;

fn main() {
    pretty_env_logger::init();

    let engine = Arc::new(Tera::new("templates/**/*").unwrap());

    let index = path!(@get /).and(Renderer::new(engine.clone(), "index.html"));

    let detailed = {
        #[derive(Debug, Serialize)]
        struct Context {
            name: String,
        }

        path!(@get / "greeting" / String /)
            .map(|name| Context { name })
            .wrap(Renderer::new(engine.clone(), "greeting.html"))
    };

    let endpoint = index.or(detailed);

    info!("Listening on http://127.0.0.1:4000");
    finchers::launch(endpoint).start("127.0.0.1:4000");
}
