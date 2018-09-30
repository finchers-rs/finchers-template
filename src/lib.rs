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
#[cfg(feature = "handlebars")]
extern crate handlebars;
extern crate http;
extern crate mime;
extern crate mime_guess;
extern crate serde;
#[cfg(feature = "tera")]
extern crate tera;

pub use self::imp::{Renderer, TemplateEndpoint, TemplateEngine};

#[cfg(feature = "handlebars")]
#[doc(no_inline)]
pub use handlebars::Handlebars;

#[cfg(feature = "tera")]
#[doc(no_inline)]
pub use tera::Tera;

mod imp {
    use finchers;
    use finchers::endpoint;
    use finchers::endpoint::wrapper::Wrapper;
    use finchers::endpoint::{ApplyContext, ApplyResult, Endpoint, IntoEndpoint};
    use finchers::error::Error;
    use finchers::output::body::ResBody;

    #[cfg(feature = "handlebars")]
    use handlebars::Handlebars;
    #[cfg(feature = "tera")]
    use tera::Tera;

    use failure::SyncFailure;
    use futures::{Future, Poll};
    use http::Response;
    use mime::Mime;
    use mime_guess::guess_mime_type;
    use serde::Serialize;

    use std::borrow::Cow;
    use std::rc::Rc;
    use std::sync::Arc;

    /// A trait representing template engine used in `Renderer`.
    #[allow(missing_docs)]
    pub trait TemplateEngine {
        type Body: ResBody;
        type Error: Into<Error>;

        fn render<T>(&self, template_name: &str, ctx: &T) -> Result<Self::Body, Self::Error>
        where
            T: Serialize;
    }

    impl<E: TemplateEngine> TemplateEngine for Box<E> {
        type Body = E::Body;
        type Error = E::Error;

        fn render<T>(&self, template_name: &str, ctx: &T) -> Result<Self::Body, Self::Error>
        where
            T: Serialize,
        {
            (**self).render(template_name, ctx)
        }
    }

    impl<E: TemplateEngine> TemplateEngine for Rc<E> {
        type Body = E::Body;
        type Error = E::Error;

        fn render<T>(&self, template_name: &str, ctx: &T) -> Result<Self::Body, Self::Error>
        where
            T: Serialize,
        {
            (**self).render(template_name, ctx)
        }
    }

    impl<E: TemplateEngine> TemplateEngine for Arc<E> {
        type Body = E::Body;
        type Error = E::Error;

        fn render<T>(&self, template_name: &str, ctx: &T) -> Result<Self::Body, Self::Error>
        where
            T: Serialize,
        {
            (**self).render(template_name, ctx)
        }
    }

    #[cfg(feature = "handlebars")]
    impl TemplateEngine for Handlebars {
        type Body = String;
        type Error = Error;

        fn render<T>(&self, template_name: &str, ctx: &T) -> Result<Self::Body, Self::Error>
        where
            T: Serialize,
        {
            Handlebars::render(self, template_name, ctx)
                .map_err(|err| finchers::error::fail(SyncFailure::new(err)))
        }
    }

    #[cfg(feature = "tera")]
    impl TemplateEngine for Tera {
        type Body = String;
        type Error = Error;

        fn render<T>(&self, template_name: &str, ctx: &T) -> Result<Self::Body, Self::Error>
        where
            T: Serialize,
        {
            Tera::render(self, template_name, ctx)
                .map_err(|err| finchers::error::fail(SyncFailure::new(err)))
        }
    }

    /// The type representing a renderer using Tera template engine.
    #[derive(Debug, Clone)]
    pub struct Renderer<T> {
        engine: T,
        name: Cow<'static, str>,
        content_type: Mime,
    }

    impl<T> Renderer<T>
    where
        T: TemplateEngine,
    {
        /// Create a new `Renderer` from the specified Tera engine and template name.
        pub fn new(engine: T, name: impl Into<Cow<'static, str>>) -> Renderer<T> {
            let name = name.into();
            let content_type = guess_mime_type(&*name);
            Renderer {
                engine,
                name,
                content_type,
            }
        }

        /// Set the content-type of generated content.
        ///
        /// By default, the value is guessed from the name of template.
        pub fn content_type(self, content_type: Mime) -> Renderer<T> {
            Renderer {
                content_type,
                ..self
            }
        }

        /// Renders a template using the specified context value.
        fn render_html<CtxT>(&self, ctx: &CtxT) -> Result<Response<T::Body>, Error>
        where
            CtxT: Serialize,
        {
            let body = self.engine.render(&self.name, ctx).map_err(Into::into)?;

            Ok(Response::builder()
                .header("content-type", self.content_type.as_ref())
                .body(body)
                .expect("should be a valid response"))
        }
    }

    impl<'a, T> IntoEndpoint<'a> for Renderer<T>
    where
        T: TemplateEngine + 'a,
    {
        type Output = (Response<T::Body>,);
        type Endpoint = TemplateEndpoint<T, endpoint::Cloned<self::dummy::DummyContext>>;

        fn into_endpoint(self) -> Self::Endpoint {
            TemplateEndpoint {
                renderer: self,
                endpoint: endpoint::cloned(Default::default()),
            }
        }
    }

    mod dummy {
        use serde::ser::{Serialize, SerializeMap, Serializer};

        #[derive(Debug, Default, Clone)]
        pub struct DummyContext {
            _priv: (),
        }

        impl Serialize for DummyContext {
            fn serialize<S>(&self, ser: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                ser.serialize_map(Some(0))?.end()
            }
        }
    }

    impl<'a, T, E, CtxT> Wrapper<'a, E> for Renderer<T>
    where
        T: TemplateEngine + 'a,
        E: Endpoint<'a, Output = (CtxT,)>,
        CtxT: Serialize,
    {
        type Output = (Response<T::Body>,);
        type Endpoint = TemplateEndpoint<T, E>;

        fn wrap(self, endpoint: E) -> Self::Endpoint {
            TemplateEndpoint {
                renderer: self,
                endpoint,
            }
        }
    }

    /// The type of endpoint which renders a Tera template with the value of specified context type.
    #[derive(Debug)]
    pub struct TemplateEndpoint<T, E> {
        renderer: Renderer<T>,
        endpoint: E,
    }

    impl<'a, T, E, CtxT> Endpoint<'a> for TemplateEndpoint<T, E>
    where
        T: TemplateEngine + 'a,
        E: Endpoint<'a, Output = (CtxT,)>,
        CtxT: Serialize,
    {
        type Output = (Response<T::Body>,);
        type Future = TemplateFuture<'a, T, E>;

        #[inline]
        fn apply(&'a self, cx: &mut ApplyContext<'_>) -> ApplyResult<Self::Future> {
            Ok(TemplateFuture {
                future: self.endpoint.apply(cx)?,
                renderer: &self.renderer,
            })
        }
    }

    #[derive(Debug)]
    pub struct TemplateFuture<'a, T: TemplateEngine + 'a, E: Endpoint<'a>> {
        future: E::Future,
        renderer: &'a Renderer<T>,
    }

    impl<'a, T, E, CtxT> Future for TemplateFuture<'a, T, E>
    where
        T: TemplateEngine + 'a,
        E: Endpoint<'a, Output = (CtxT,)>,
        CtxT: Serialize,
    {
        type Item = (Response<T::Body>,);
        type Error = Error;

        #[inline]
        fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
            let (ctx,) = try_ready!(self.future.poll());
            self.renderer
                .render_html(&ctx)
                .map(|response| (response,).into())
        }
    }
}
