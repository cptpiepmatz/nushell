use heck::ToSnakeCase;
use proc_macro2::{Ident, Span};
use quote::quote;

pub fn make() -> proc_macro2::TokenStream {
    let experimental_options_fields = nu_experimental::ALL.into_iter().map(|option| {
        let ident = Ident::new(
            option.identifier().to_snake_case().as_str(),
            Span::call_site(),
        );
        let doc = option.description();

        quote! {
            #[doc = #doc]
            pub #ident: ::std::option::Option<bool>,
        }
    });

    let experimental_options_display = nu_experimental::ALL.into_iter().map(|option| {
        let ident = Ident::new(option.identifier().to_snake_case().as_str(), Span::call_site());
        let name = option.identifier();
        quote! {
            if let Some(value) = &self.#ident {
                if first {
                    first = false;
                    f.write_str(", ")?;
                }

                f.write_fmt(::std::format_args!("{}={}", #name, value))?;
            }
        }
    });

    quote! {
        #[doc = "Requested experimental options."]
        #[doc = ""]
        #[doc = "The type is generated from [`nu_experimental::ALL`]. "]
        #[deprecated = "Do not construct this type manually, the `nu_test_support::harness::test` macro uses this internally."]
        #[derive(
            ::std::fmt::Debug, 
            ::std::cmp::PartialEq, 
            ::std::cmp::Eq, 
            ::std::hash::Hash
        )]
        pub struct RequestedExperimentalOptions {
            #(#experimental_options_fields)*
        }

        impl ::std::fmt::Display for RequestedExperimentalOptions {
            fn fmt(
                &self, 
                f: &mut ::std::fmt::Formatter<'_>
            ) -> ::std::result::Result<(), ::std::fmt::Error> {
                let mut first = false;
                let mut out = ::std::string::String::new();
                #(#experimental_options_display)*
                ::std::result::Result::Ok(())
            }
        }
    }
}
