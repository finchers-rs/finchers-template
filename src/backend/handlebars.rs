#![cfg(feature = "use-handlebars")]

use renderer::{renderer, Engine, EngineImpl, Renderer};

use failure::SyncFailure;
use handlebars::Handlebars;
use http::header::HeaderValue;
use mime::Mime;
use mime_guess::guess_mime_type;
use serde::Serialize;

pub trait AsHandlebarsRegistry {
    fn as_ref(&self) -> &Handlebars;
}

impl AsHandlebarsRegistry for Handlebars {
    fn as_ref(&self) -> &Handlebars {
        self
    }
}

impl<T: AsHandlebarsRegistry> AsHandlebarsRegistry for Box<T> {
    fn as_ref(&self) -> &Handlebars {
        (**self).as_ref()
    }
}

impl<T: AsHandlebarsRegistry> AsHandlebarsRegistry for ::std::rc::Rc<T> {
    fn as_ref(&self) -> &Handlebars {
        (**self).as_ref()
    }
}

impl<T: AsHandlebarsRegistry> AsHandlebarsRegistry for ::std::sync::Arc<T> {
    fn as_ref(&self) -> &Handlebars {
        (**self).as_ref()
    }
}

pub fn handlebars<H>(handlebars: H, name: impl Into<String>) -> Renderer<HandlebarsEngine<H>>
where
    H: AsHandlebarsRegistry,
{
    let name = name.into();
    let content_type = guess_mime_type(&name)
        .as_ref()
        .parse()
        .expect("should be a valid header value");
    renderer(HandlebarsEngine {
        handlebars,
        name,
        content_type,
    })
}

#[derive(Debug)]
pub struct HandlebarsEngine<H> {
    handlebars: H,
    name: String,
    content_type: HeaderValue,
}

impl<H> HandlebarsEngine<H> {
    pub fn set_content_type(&mut self, content_type: Mime) {
        self.content_type = content_type
            .as_ref()
            .parse()
            .expect("should be a valid header value");
    }
}

impl<H, T: Serialize> Engine<T> for HandlebarsEngine<H> where H: AsHandlebarsRegistry {}

impl<H, CtxT: Serialize> EngineImpl<CtxT> for HandlebarsEngine<H>
where
    H: AsHandlebarsRegistry,
{
    type Body = String;
    type Error = SyncFailure<::handlebars::RenderError>;

    fn content_type_hint(&self, _: &CtxT) -> Option<HeaderValue> {
        Some(self.content_type.clone())
    }

    fn render(&self, value: CtxT) -> Result<Self::Body, Self::Error> {
        self.handlebars
            .as_ref()
            .render(&self.name, &value)
            .map_err(SyncFailure::new)
    }
}

#[test]
fn test_handlebars() {
    use std::sync::Arc;

    #[derive(Debug, Serialize)]
    struct Context {
        name: String,
    }

    let mut inner = Handlebars::new();
    inner
        .register_template_string("index.html", "{{ name }}")
        .unwrap();

    let value = Context {
        name: "Alice".into(),
    };

    let renderer = handlebars(Arc::new(inner), "index.html");
    let body = renderer.engine.render(value).unwrap();
    assert_eq!(body, "Alice");
}
