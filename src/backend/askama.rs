#![cfg(feature = "use-askama")]

use askama::Template;
use http::header::HeaderValue;
use mime_guess::get_mime_type_str;
use renderer::{renderer, Engine, EngineImpl, Renderer};

pub fn askama() -> Renderer<AskamaEngine> {
    renderer(AskamaEngine { _priv: () })
}

#[derive(Debug)]
pub struct AskamaEngine {
    _priv: (),
}

impl<CtxT: Template> Engine<CtxT> for AskamaEngine {}

impl<CtxT: Template> EngineImpl<CtxT> for AskamaEngine {
    type Body = String;
    type Error = ::askama::Error;

    // FIXME: cache parsed value
    fn content_type_hint(&self, value: &CtxT) -> Option<HeaderValue> {
        let ext = value.extension()?;
        get_mime_type_str(ext)?.parse().ok()
    }

    fn render(&self, value: CtxT) -> Result<Self::Body, Self::Error> {
        value.render()
    }
}

#[test]
fn test_askama() {
    use askama::Error;
    use std::fmt;

    #[derive(Debug)]
    struct Context {
        name: String,
    }

    impl Template for Context {
        fn render_into(&self, writer: &mut dyn fmt::Write) -> Result<(), Error> {
            write!(writer, "{}", self.name).map_err(Into::into)
        }

        fn extension(&self) -> Option<&str> {
            Some("html")
        }
    }

    let value = Context {
        name: "Alice".into(),
    };

    let renderer = askama();
    let body = renderer.engine.render(value).unwrap();
    assert_eq!(body, "Alice");
}
