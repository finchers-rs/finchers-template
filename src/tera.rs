extern crate tera;

pub use self::imp::{Renderer, TemplateEndpoint};
#[doc(no_inline)]
pub use self::tera::*;

mod imp {
    use finchers;
    use finchers::endpoint;
    use finchers::endpoint::wrapper::Wrapper;
    use finchers::endpoint::{ApplyContext, ApplyResult, Endpoint, IntoEndpoint};
    use finchers::error::Error;

    use super::tera::Tera;

    use failure::SyncFailure;
    use futures::{Future, Poll};
    use http::Response;
    use mime::Mime;
    use mime_guess::guess_mime_type;
    use serde::Serialize;
    use std::borrow::Cow;

    /// The type representing a renderer using Tera template engine.
    #[derive(Debug)]
    pub struct Renderer<T> {
        engine: T,
        name: Cow<'static, str>,
        content_type: Mime,
    }

    impl<T> Renderer<T>
    where
        T: AsRef<Tera>,
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

        /// Renders a Tera template using the specified context value.
        fn render_response<CtxT>(&self, ctx: &CtxT) -> Result<Response<String>, Error>
        where
            CtxT: Serialize,
        {
            let body = self
                .engine
                .as_ref()
                .render(&self.name, ctx)
                .map_err(|err| finchers::error::fail(SyncFailure::new(err)))?;

            Ok(Response::builder()
                .header("content-type", self.content_type.as_ref())
                .body(body)
                .expect("should be a valid response"))
        }
    }

    impl<'a, T> IntoEndpoint<'a> for Renderer<T>
    where
        T: AsRef<Tera> + 'a,
    {
        type Output = (Response<String>,);
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
        T: AsRef<Tera> + 'a,
        E: Endpoint<'a, Output = (CtxT,)>,
        CtxT: Serialize,
    {
        type Output = (Response<String>,);
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
        T: AsRef<Tera> + 'a,
        E: Endpoint<'a, Output = (CtxT,)>,
        CtxT: Serialize,
    {
        type Output = (Response<String>,);
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
    pub struct TemplateFuture<'a, T: AsRef<Tera> + 'a, E: Endpoint<'a>> {
        future: E::Future,
        renderer: &'a Renderer<T>,
    }

    impl<'a, T, E, CtxT> Future for TemplateFuture<'a, T, E>
    where
        T: AsRef<Tera>,
        E: Endpoint<'a, Output = (CtxT,)>,
        CtxT: Serialize,
    {
        type Item = (Response<String>,);
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
