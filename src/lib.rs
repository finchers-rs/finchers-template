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
extern crate mime;
extern crate mime_guess;
extern crate serde;

#[cfg(feature = "handlebars")]
pub mod handlebars;
#[cfg(feature = "tera")]
pub mod tera;
