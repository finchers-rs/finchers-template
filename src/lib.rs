//! Template support for Finchers

// master
#![doc(html_root_url = "https://finchers-rs.github.io/finchers-template-tera")]
// released
//#![doc(html_root_url = "https://docs.rs/finchers-template-tera/0.1.0")]
#![warn(
    missing_docs,
    missing_debug_implementations,
    nonstandard_style,
    rust_2018_idioms,
    unused,
)]
//#![warn(rust_2018_compatibility)]
#![cfg_attr(feature = "strict", deny(warnings))]
#![cfg_attr(feature = "strict", doc(test(attr(deny(warnings)))))]

extern crate failure;
extern crate finchers;
#[macro_use]
extern crate futures;
extern crate http;
extern crate serde;
extern crate tera;

use finchers::endpoint::wrapper::Wrapper;
use finchers::endpoint::{Context as EndpointContext, Endpoint, EndpointResult, IntoEndpoint};
use finchers::error::Error;

use failure::SyncFailure;
use futures::{Future, Poll};
use http::Response;
use serde::Serialize;
use std::borrow::Cow;
use std::sync::Arc;
use tera::Tera;

pub fn template(engine: Tera) -> Template {
    Template::new(engine)
}

#[derive(Debug)]
pub struct Template {
    engine: Arc<Tera>,
}

impl Template {
    pub fn new(engine: Tera) -> Template {
        Template {
            engine: Arc::new(engine),
        }
    }

    pub fn to_renderer(&self, name: impl Into<Cow<'static, str>>) -> TemplateRenderer {
        TemplateRenderer {
            engine: self.engine.clone(),
            name: name.into(),
        }
    }
}

#[derive(Debug)]
pub struct TemplateRenderer {
    engine: Arc<Tera>,
    name: Cow<'static, str>,
}

impl<'a, E, CtxT> Wrapper<'a, E> for TemplateRenderer
where
    E: Endpoint<'a, Output = (CtxT,)>,
    CtxT: Serialize,
{
    type Output = (Response<String>,);
    type Endpoint = TemplateEndpoint<E>;

    fn wrap(self, endpoint: E) -> Self::Endpoint {
        TemplateEndpoint {
            endpoint,
            engine: self.engine,
            name: self.name,
        }
    }
}

impl<'a> IntoEndpoint<'a> for TemplateRenderer {
    type Output = (Response<String>,);
    type Endpoint = TemplateEndpoint<finchers::endpoint::Value<()>>;

    fn into_endpoint(self) -> Self::Endpoint {
        TemplateEndpoint {
            endpoint: finchers::endpoint::value(()),
            engine: self.engine,
            name: self.name,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TemplateEndpoint<E> {
    endpoint: E,
    engine: Arc<Tera>,
    name: Cow<'static, str>,
}

impl<'a, E, CtxT> Endpoint<'a> for TemplateEndpoint<E>
where
    E: Endpoint<'a, Output = (CtxT,)>,
    CtxT: Serialize,
{
    type Output = (Response<String>,);
    type Future = TemplateFuture<'a, E>;

    #[inline]
    fn apply(&'a self, cx: &mut EndpointContext<'_>) -> EndpointResult<Self::Future> {
        Ok(TemplateFuture {
            future: self.endpoint.apply(cx)?,
            endpoint: self,
        })
    }
}

#[doc(hidden)]
#[derive(Debug)]
pub struct TemplateFuture<'a, E: Endpoint<'a>> {
    future: E::Future,
    endpoint: &'a TemplateEndpoint<E>,
}

impl<'a, E, CtxT> Future for TemplateFuture<'a, E>
where
    E: Endpoint<'a, Output = (CtxT,)>,
    CtxT: Serialize,
{
    type Item = (Response<String>,);
    type Error = Error;

    #[inline]
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let (ctx,) = try_ready!(self.future.poll());
        self.endpoint
            .engine
            .render(&self.endpoint.name, &ctx)
            .map(|body| {
                let response = Response::builder()
                    .header("content-type", "text/html; charset=utf-8")
                    .body(body)
                    .expect("should be a valid response");
                (response,).into()
            }).map_err(|err| finchers::error::fail(SyncFailure::new(err)))
    }
}
