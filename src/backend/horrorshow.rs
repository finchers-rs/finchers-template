#![cfg(feature = "use-horrorshow")]

use horrorshow::Template;

use renderer::{renderer, Engine, EngineImpl, Renderer};

pub fn horrorshow() -> Renderer<HorrorshowEngine> {
    renderer(HorrorshowEngine { _priv: () })
}

#[derive(Debug)]
pub struct HorrorshowEngine {
    _priv: (),
}

impl<CtxT: Template> Engine<CtxT> for HorrorshowEngine {}

impl<CtxT: Template> EngineImpl<CtxT> for HorrorshowEngine {
    type Body = String;
    type Error = ::horrorshow::Error;

    fn render(&self, value: CtxT) -> Result<Self::Body, Self::Error> {
        value.into_string()
    }
}

#[test]
fn test_horrorshow() {
    let value = {
        html!{
            p: "Alice";
        }
    };

    let renderer = horrorshow();
    let body = renderer.engine.render(value).unwrap();
    assert_eq!(body, "<p>Alice</p>");
}
