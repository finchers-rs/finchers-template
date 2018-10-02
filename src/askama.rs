//! Components for supporting askama.

extern crate askama;

#[doc(no_inline)]
pub use self::askama::Template;
pub use self::imp::{renderer, RenderEndpoint, Renderer};

mod imp {
    use super::askama::Template;

    use finchers::endpoint::wrapper::Wrapper;
    use finchers::endpoint::{ApplyContext, ApplyResult, Endpoint};
    use finchers::error;
    use finchers::error::Error;

    use futures::{Future, Poll};
    use http::header;
    use http::header::HeaderValue;
    use http::Response;
    use mime;
    use mime::Mime;

    /// Create a `Renderer` for rendering the value of context type which implements
    /// `askama::Template`.
    pub fn renderer() -> Renderer {
        Renderer {
            content_type: mime::TEXT_HTML_UTF_8.as_ref().parse().unwrap(),
        }
    }

    /// The type for modifying the result rendered by Askama to an HTTP response.
    #[derive(Debug)]
    pub struct Renderer {
        content_type: HeaderValue,
    }

    impl Renderer {
        /// Sets the content type of generated HTTP response.
        ///
        /// The default value is `text/html; charset=utf-8`.
        pub fn content_type(self, content_type: Mime) -> Renderer {
            let content_type = content_type
                .as_ref()
                .parse()
                .expect("the MIME value should be a valid header value");
            Renderer {
                content_type,
                ..self
            }
        }

        fn render_response<T>(&self, ctx: &T) -> Result<Response<String>, super::askama::Error>
        where
            T: Template,
        {
            let mut response = ctx.render().map(Response::new)?;
            response
                .headers_mut()
                .insert(header::CONTENT_TYPE, self.content_type.clone());
            Ok(response)
        }
    }

    impl<'a, E, CtxT> Wrapper<'a, E> for Renderer
    where
        E: Endpoint<'a, Output = (CtxT,)>,
        CtxT: Template,
    {
        type Output = (Response<String>,);
        type Endpoint = RenderEndpoint<E>;

        fn wrap(self, endpoint: E) -> Self::Endpoint {
            RenderEndpoint {
                endpoint,
                renderer: self,
            }
        }
    }

    /// An endpoint which renders the output of inner endpoint and
    /// convert it into an HTTP response.
    #[derive(Debug)]
    pub struct RenderEndpoint<E> {
        endpoint: E,
        renderer: Renderer,
    }

    impl<'a, E, CtxT> Endpoint<'a> for RenderEndpoint<E>
    where
        E: Endpoint<'a, Output = (CtxT,)>,
        CtxT: Template,
    {
        type Output = (Response<String>,);
        type Future = RenderFuture<'a, E>;

        fn apply(&'a self, cx: &mut ApplyContext<'_>) -> ApplyResult<Self::Future> {
            Ok(RenderFuture {
                future: self.endpoint.apply(cx)?,
                endpoint: self,
            })
        }
    }

    // not a public API.
    #[derive(Debug)]
    pub struct RenderFuture<'a, E: Endpoint<'a>> {
        future: E::Future,
        endpoint: &'a RenderEndpoint<E>,
    }

    impl<'a, E, CtxT> Future for RenderFuture<'a, E>
    where
        E: Endpoint<'a, Output = (CtxT,)>,
        CtxT: Template,
    {
        type Item = (Response<String>,);
        type Error = Error;

        fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
            let (ctx,) = try_ready!(self.future.poll());
            self.endpoint
                .renderer
                .render_response(&ctx)
                .map(|response| (response,).into())
                .map_err(error::fail)
        }
    }
}
