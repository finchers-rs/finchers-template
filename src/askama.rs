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
    use http::Response;
    use mime;
    use mime::Mime;

    #[allow(missing_docs)]
    pub fn renderer() -> Renderer {
        Renderer {
            content_type: mime::TEXT_HTML_UTF_8,
        }
    }

    #[allow(missing_docs)]
    #[derive(Debug)]
    pub struct Renderer {
        content_type: Mime,
    }

    impl Renderer {
        fn render_response<T>(&self, ctx: &T) -> Result<Response<String>, Error>
        where
            T: Template,
        {
            ctx.render()
                .map(|body| {
                    Response::builder()
                        .header("content-type", mime::TEXT_HTML_UTF_8.as_ref())
                        .body(body)
                        .unwrap()
                }).map_err(error::fail)
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

    #[allow(missing_docs)]
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
        }
    }
}
