#![cfg(feature = "handlebars")]

#[macro_use]
extern crate finchers;
extern crate finchers_template;
#[macro_use]
extern crate log;
extern crate pretty_env_logger;
#[macro_use]
extern crate serde;
extern crate handlebars;

use finchers::prelude::*;
use finchers_template::handlebars::Renderer;

use handlebars::Handlebars;
use std::sync::Arc;

fn main() {
    pretty_env_logger::init();

    let mut engine = Handlebars::new();
    engine
        .register_template_string(
            "index.html",
            "<!doctype html>\n
             <html>\n
               <head>\n
                 <meta charset=\"utf-8\" />\n
                 <title>Index</title>\n
               </head>\n
               <body>\n
                  <p>Hello.</p>\n
               </body>\n
             </html>",
        ).unwrap();
    engine
        .register_template_string(
            "greeting.html",
            "<!doctype html>\n
             <html>\n
               <head>\n
                 <meta charset=\"utf-8\" />\n
                 <title>Greeting</title>\n
               </head>\n
               <body>\n
                 Hello, {{ name }}.\n
               </body>\n
             </html>",
        ).unwrap();
    let engine = Arc::new(engine);

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
