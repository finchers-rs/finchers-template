#![allow(missing_docs)]

use finchers::endpoint::wrapper::Wrapper;
use finchers::endpoint::{ApplyContext, ApplyResult, Endpoint};
use finchers::error;
use finchers::output::body::ResBody;

use std::fmt;
use std::marker::PhantomData;

use failure;
use futures::{Async, Future, Poll};
use http::header;
use http::header::HeaderValue;
use http::Response;

pub trait Engine<T>: EngineImpl<T> {}

pub trait EngineImpl<T> {
    type Body: ResBody;
    type Error: Into<failure::Error>;

    #[allow(unused_variables)]
    fn content_type_hint(&self, value: &T) -> Option<HeaderValue> {
        None
    }

    fn render(&self, value: T) -> Result<Self::Body, Self::Error>;
}

pub fn renderer<Eng>(engine: Eng) -> Renderer<Eng> {
    Renderer { engine }
}

#[derive(Debug)]
pub struct Renderer<Eng> {
    pub(crate) engine: Eng,
}

impl<Eng> Renderer<Eng> {
    fn render_response<T>(&self, value: T) -> error::Result<Response<Eng::Body>>
    where
        Eng: Engine<T>,
    {
        let content_type = self.engine.content_type_hint(&value).unwrap_or_else(|| {
            lazy_static! {
                static ref DEF: HeaderValue = HeaderValue::from_static("text/html; charset=utf-8");
            }
            DEF.clone()
        });

        self.engine
            .render(value)
            .map(|body| {
                let mut response = Response::new(body);
                response
                    .headers_mut()
                    .insert(header::CONTENT_TYPE, content_type);
                response
            }).map_err(|err| error::Error::from(err.into()))
    }
}

impl<'a, E, Eng, T> Wrapper<'a, E> for Renderer<Eng>
where
    E: Endpoint<'a, Output = (T,)>,
    Eng: Engine<T> + 'a,
    T: 'a,
{
    type Output = (Response<Eng::Body>,);
    type Endpoint = RenderEndpoint<E, Eng, T>;

    fn wrap(self, endpoint: E) -> Self::Endpoint {
        RenderEndpoint {
            endpoint,
            renderer: self,
            _marker: PhantomData,
        }
    }
}

pub struct RenderEndpoint<E, Eng, T>
where
    Eng: Engine<T>,
{
    endpoint: E,
    renderer: Renderer<Eng>,
    _marker: PhantomData<fn() -> T>,
}

impl<E, Eng, T> fmt::Debug for RenderEndpoint<E, Eng, T>
where
    E: fmt::Debug,
    Eng: Engine<T> + fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RenderEndpoint")
            .field("endpoint", &self.endpoint)
            .field("renderer", &self.renderer)
            .finish()
    }
}

impl<'a, E, Eng, T> Endpoint<'a> for RenderEndpoint<E, Eng, T>
where
    E: Endpoint<'a, Output = (T,)>,
    Eng: Engine<T> + 'a,
    T: 'a,
{
    type Output = (Response<Eng::Body>,);
    type Future = RenderFuture<'a, E, Eng, T>;

    fn apply(&'a self, cx: &mut ApplyContext<'_>) -> ApplyResult<Self::Future> {
        Ok(RenderFuture {
            future: self.endpoint.apply(cx)?,
            renderer: &self.renderer,
            _marker: PhantomData,
        })
    }
}

pub struct RenderFuture<'a, E: Endpoint<'a>, Eng: 'a, T: 'a>
where
    Eng: Engine<T>,
{
    future: E::Future,
    renderer: &'a Renderer<Eng>,
    _marker: PhantomData<fn() -> T>,
}

impl<'a, E, Eng, T> fmt::Debug for RenderFuture<'a, E, Eng, T>
where
    E: Endpoint<'a> + fmt::Debug,
    E::Future: fmt::Debug,
    Eng: Engine<T> + fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RenderFuture")
            .field("future", &self.future)
            .field("renderer", &self.renderer)
            .finish()
    }
}

impl<'a, E, Eng, T> Future for RenderFuture<'a, E, Eng, T>
where
    E: Endpoint<'a, Output = (T,)>,
    Eng: Engine<T> + 'a,
{
    type Item = (Response<Eng::Body>,);
    type Error = error::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let (value,) = try_ready!(self.future.poll());
        self.renderer
            .render_response(value)
            .map(|response| Async::Ready((response,)))
    }
}

#[cfg(test)]
mod tests {
    use super::{renderer, Engine, EngineImpl};
    use finchers::error;
    use finchers::prelude::*;
    use finchers::test;
    use std::string::ToString;

    #[test]
    fn test_renderer() {
        struct DummyEngine;
        impl<T: ToString> Engine<T> for DummyEngine {}
        impl<T: ToString> EngineImpl<T> for DummyEngine {
            type Body = String;
            type Error = error::Never;
            fn render(&self, value: T) -> Result<Self::Body, Self::Error> {
                Ok(value.to_string())
            }
        }

        let mut runner = test::runner({
            endpoint::syntax::verb::get()
                .and(endpoint::syntax::param::<String>())
                .and(endpoint::syntax::eos())
                .wrap(renderer(DummyEngine))
        });

        let response = runner.perform("/Amaterasu").unwrap();
        assert_eq!(response.status().as_u16(), 200);
        assert_matches!(
            response.headers().get("content-type"),
            Some(h) if h == "text/html; charset=utf-8"
        );
        assert_eq!(response.body().to_utf8().unwrap(), "Amaterasu");
    }

}
