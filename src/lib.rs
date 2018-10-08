// FIXME: remove this feature gate as soon as the rustc version used in docs.rs is updated
#![cfg_attr(finchers_inject_extern_prelude, feature(extern_prelude))]

//! Template support for Finchers

#![doc(html_root_url = "https://docs.rs/finchers-template/0.1.1")]
#![warn(
    missing_docs,
    missing_debug_implementations,
    nonstandard_style,
    rust_2018_idioms,
    unused,
)]
//#![warn(rust_2018_compatibility)]
#![cfg_attr(test, deny(warnings))]
#![cfg_attr(test, doc(test(attr(deny(warnings)))))]

#[cfg(any(feature = "tera", feature = "handlebars"))]
extern crate failure;
extern crate finchers;
#[macro_use]
extern crate futures;
extern crate http;
extern crate mime;
extern crate mime_guess;
extern crate serde;

#[cfg(feature = "handlebars")]
extern crate handlebars;

#[cfg(feature = "tera")]
extern crate tera;

#[allow(deprecated)]
pub use self::imp::TemplateEndpoint;
pub use self::imp::{renderer, RenderEndpoint, Renderer, TemplateEngine};

#[cfg(feature = "handlebars")]
#[doc(no_inline)]
pub use handlebars::Handlebars;

#[cfg(feature = "tera")]
#[doc(no_inline)]
pub use tera::Tera;

#[cfg(feature = "askama")]
pub mod askama;

#[cfg(feature = "horrorshow")]
pub mod horrorshow;

mod imp {
    use finchers::endpoint;
    use finchers::endpoint::wrapper::Wrapper;
    use finchers::endpoint::{ApplyContext, ApplyResult, Endpoint, IntoEndpoint};
    use finchers::error::Error;
    use finchers::output::body::ResBody;

    #[cfg(feature = "handlebars")]
    use handlebars::Handlebars;
    #[cfg(feature = "tera")]
    use tera::Tera;

    use futures::{Future, Poll};
    use http::header;
    use http::header::HeaderValue;
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
            use failure::SyncFailure;
            use finchers;

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
            use failure::SyncFailure;
            use finchers;

            Tera::render(self, template_name, ctx)
                .map_err(|err| finchers::error::fail(SyncFailure::new(err)))
        }
    }

    /// Create a new `Renderer` from the specified template engine and template name.
    pub fn renderer<T>(engine: T, name: impl Into<Cow<'static, str>>) -> Renderer<T>
    where
        T: TemplateEngine,
    {
        let name = name.into();
        let content_type = HeaderValue::from_shared(guess_mime_type(&*name).as_ref().into())
            .expect("should be a valid header value");
        Renderer {
            engine,
            name,
            content_type,
        }
    }

    /// The type representing a renderer using the specified template engine.
    #[derive(Debug, Clone)]
    pub struct Renderer<T> {
        engine: T,
        name: Cow<'static, str>,
        content_type: HeaderValue,
    }

    impl<T> Renderer<T>
    where
        T: TemplateEngine,
    {
        #[doc(hidden)]
        #[deprecated(note = "use `renderer()` instead.")]
        pub fn new(engine: T, name: impl Into<Cow<'static, str>>) -> Renderer<T> {
            renderer(engine, name)
        }

        /// Set the content-type of generated content.
        ///
        /// By default, the value is guessed from the name of template.
        pub fn content_type(self, content_type: Mime) -> Renderer<T> {
            Renderer {
                content_type: HeaderValue::from_shared(content_type.as_ref().into())
                    .expect("should be a valid header value"),
                ..self
            }
        }

        /// Renders a template using the specified context value.
        fn render_response<CtxT>(&self, ctx: &CtxT) -> Result<Response<T::Body>, Error>
        where
            CtxT: Serialize,
        {
            let mut response = self
                .engine
                .render(&self.name, ctx)
                .map(Response::new)
                .map_err(Into::into)?;
            response
                .headers_mut()
                .insert(header::CONTENT_TYPE, self.content_type.clone());
            Ok(response)
        }
    }

    impl<'a, T> IntoEndpoint<'a> for Renderer<T>
    where
        T: TemplateEngine + 'a,
    {
        type Output = (Response<T::Body>,);
        type Endpoint = RenderEndpoint<T, endpoint::Cloned<self::dummy::DummyContext>>;

        fn into_endpoint(self) -> Self::Endpoint {
            RenderEndpoint {
                renderer: self,
                endpoint: endpoint::cloned(Default::default()),
            }
        }
    }

    mod dummy {
        use serde::ser::{Serialize, SerializeMap, Serializer};

        #[derive(Debug, Default, Clone, Copy)]
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
        type Endpoint = RenderEndpoint<T, E>;

        fn wrap(self, endpoint: E) -> Self::Endpoint {
            RenderEndpoint {
                renderer: self,
                endpoint,
            }
        }
    }

    #[doc(hidden)]
    #[deprecated(since = "0.1.1", note = "renamed to `RenderEndpoint<T, E>")]
    pub type TemplateEndpoint<T, E> = RenderEndpoint<T, E>;

    /// The type of endpoint which renders a Tera template with the value of specified context type.
    #[derive(Debug)]
    pub struct RenderEndpoint<T, E> {
        renderer: Renderer<T>,
        endpoint: E,
    }

    impl<'a, T, E, CtxT> Endpoint<'a> for RenderEndpoint<T, E>
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
                .render_response(&ctx)
                .map(|response| (response,).into())
        }
    }
}
