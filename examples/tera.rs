#![cfg(feature = "tera")]

#[macro_use]
extern crate finchers;
extern crate finchers_template;
#[macro_use]
extern crate log;
extern crate pretty_env_logger;
#[macro_use]
extern crate serde;
extern crate tera;

use finchers::prelude::*;
use finchers_template::Renderer;

use std::sync::Arc;
use tera::Tera;

fn main() {
    pretty_env_logger::init();

    let mut engine = Tera::default();
    engine
        .add_raw_template(
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
        .add_raw_template(
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
