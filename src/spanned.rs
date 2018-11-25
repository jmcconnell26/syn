//! A trait that can provide the `Span` of the complete contents of a syntax
//! tree node.
//!
//! *This module is available if Syn is built with both the `"parsing"` and
//! `"printing"` features.*
//!
//! # Example
//!
//! Suppose in a procedural macro we have a [`Type`] that we want to assert
//! implements the [`Sync`] trait. Maybe this is the type of one of the fields
//! of a struct for which we are deriving a trait implementation, and we need to
//! be able to pass a reference to one of those fields across threads.
//!
//! [`Type`]: ../enum.Type.html
//! [`Sync`]: https://doc.rust-lang.org/std/marker/trait.Sync.html
//!
//! If the field type does *not* implement `Sync` as required, we want the
//! compiler to report an error pointing out exactly which type it was.
//!
//! The following macro code takes a variable `ty` of type `Type` and produces a
//! static assertion that `Sync` is implemented for that type.
//!
//! ```
//! # extern crate proc_macro;
//! # extern crate proc_macro2;
//! # extern crate syn;
//! #
//! #[macro_use]
//! extern crate quote;
//!
//! use proc_macro::TokenStream;
//! use proc_macro2::Span;
//! use syn::Type;
//! use syn::spanned::Spanned;
//!
//! # const IGNORE_TOKENS: &str = stringify! {
//! #[proc_macro_derive(MyMacro)]
//! # };
//! pub fn my_macro(input: TokenStream) -> TokenStream {
//!     # let ty = get_a_type();
//!     /* ... */
//!
//!     let assert_sync = quote_spanned! {ty.span()=>
//!         struct _AssertSync where #ty: Sync;
//!     };
//!
//!     /* ... */
//!     # input
//! }
//! #
//! # fn get_a_type() -> Type {
//! #     unimplemented!()
//! # }
//! #
//! # fn main() {}
//! ```
//!
//! By inserting this `assert_sync` fragment into the output code generated by
//! our macro, the user's code will fail to compile if `ty` does not implement
//! `Sync`. The errors they would see look like the following.
//!
//! ```text
//! error[E0277]: the trait bound `*const i32: std::marker::Sync` is not satisfied
//!   --> src/main.rs:10:21
//!    |
//! 10 |     bad_field: *const i32,
//!    |                ^^^^^^^^^^ `*const i32` cannot be shared between threads safely
//! ```
//!
//! In this technique, using the `Type`'s span for the error message makes the
//! error appear in the correct place underlining the right type.

use proc_macro2::{Span, TokenStream};
use quote::ToTokens;

/// A trait that can provide the `Span` of the complete contents of a syntax
/// tree node.
///
/// This trait is automatically implemented for all types that implement
/// [`ToTokens`] from the `quote` crate. It is sealed and cannot be implemented
/// outside of the Syn crate other than by implementing `ToTokens`.
///
/// [`ToTokens`]: https://docs.rs/quote/0.6/quote/trait.ToTokens.html
///
/// See the [module documentation] for an example.
///
/// [module documentation]: index.html
///
/// *This trait is available if Syn is built with both the `"parsing"` and
/// `"printing"` features.*
pub trait Spanned: private::Sealed {
    /// Returns a `Span` covering the complete contents of this syntax tree
    /// node, or [`Span::call_site()`] if this node is empty.
    ///
    /// [`Span::call_site()`]: https://docs.rs/proc-macro2/0.4/proc_macro2/struct.Span.html#method.call_site
    fn span(&self) -> Span;
}

mod private {
    use quote::ToTokens;
    pub trait Sealed {}
    impl<T: ToTokens> Sealed for T {}
}

impl<T> Spanned for T
where
    T: ToTokens,
{
    fn span(&self) -> Span {
        join_spans(self.into_token_stream())
    }
}

fn join_spans(tokens: TokenStream) -> Span {
    let mut iter = tokens.into_iter().filter_map(|tt| {
        // FIXME: This shouldn't be required, since optimally spans should
        // never be invalid. This filter_map can probably be removed when
        // https://github.com/rust-lang/rust/issues/43081 is resolved.
        let span = tt.span();
        let debug = format!("{:?}", span);
        if debug.ends_with("bytes(0..0)") {
            None
        } else {
            Some(span)
        }
    });

    let mut joined = match iter.next() {
        Some(span) => span,
        None => return Span::call_site(),
    };

    #[cfg(procmacro2_semver_exempt)]
    {
        for next in iter {
            if let Some(span) = joined.join(next) {
                joined = span;
            }
        }
    }

    #[cfg(not(procmacro2_semver_exempt))]
    {
        // We can't join spans without procmacro2_semver_exempt so just grab the
        // first one.
        joined = joined;
    }

    joined
}
