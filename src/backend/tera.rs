#![cfg(feature = "use-tera")]

use failure::SyncFailure;
use http::header::HeaderValue;
use mime::Mime;
use mime_guess::guess_mime_type;
use serde::Serialize;
use tera::Tera;

use renderer::{renderer, Engine, EngineImpl, Renderer};

pub trait AsTera {
    fn as_ref(&self) -> &Tera;
}

impl AsTera for Tera {
    fn as_ref(&self) -> &Tera {
        self
    }
}

impl<T: AsTera> AsTera for Box<T> {
    fn as_ref(&self) -> &Tera {
        (**self).as_ref()
    }
}

impl<T: AsTera> AsTera for ::std::rc::Rc<T> {
    fn as_ref(&self) -> &Tera {
        (**self).as_ref()
    }
}

impl<T: AsTera> AsTera for ::std::sync::Arc<T> {
    fn as_ref(&self) -> &Tera {
        (**self).as_ref()
    }
}

pub fn tera<T>(tera: T, name: impl Into<String>) -> Renderer<TeraEngine<T>>
where
    T: AsTera,
{
    let name = name.into();
    let content_type = guess_mime_type(&name)
        .as_ref()
        .parse()
        .expect("should be a valid header value");
    renderer(TeraEngine {
        tera,
        name,
        content_type,
    })
}

#[derive(Debug)]
pub struct TeraEngine<T> {
    tera: T,
    name: String,
    content_type: HeaderValue,
}

impl<T> TeraEngine<T> {
    pub fn set_content_type(&mut self, content_type: Mime) {
        self.content_type = content_type
            .as_ref()
            .parse()
            .expect("should be a valid header value");
    }
}

impl<T, CtxT: Serialize> Engine<CtxT> for TeraEngine<T> where T: AsTera {}

impl<T, CtxT: Serialize> EngineImpl<CtxT> for TeraEngine<T>
where
    T: AsTera,
{
    type Body = String;
    type Error = SyncFailure<::tera::Error>;

    fn content_type_hint(&self, _: &CtxT) -> Option<HeaderValue> {
        Some(self.content_type.clone())
    }

    fn render(&self, value: CtxT) -> Result<Self::Body, Self::Error> {
        self.tera
            .as_ref()
            .render(&self.name, &value)
            .map_err(SyncFailure::new)
    }
}

#[test]
fn test_tera() {
    use std::sync::Arc;

    #[derive(Debug, Serialize)]
    struct Context {
        name: String,
    }

    let mut tera = Tera::default();
    tera.add_raw_template("index.html", "{{ name }}").unwrap();

    let value = Context {
        name: "Alice".into(),
    };

    let renderer = self::tera(Arc::new(tera), "index.html");
    let body = renderer.engine.render(value).unwrap();
    assert_eq!(body, "Alice");
}
