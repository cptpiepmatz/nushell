use heck::{ToShoutySnakeCase, ToSnakeCase};
use quote::{format_ident, quote};
use syn::{ItemFn, ReturnType};

pub fn test(item_fn: ItemFn) -> proc_macro2::TokenStream {
    let fn_ident = &item_fn.sig.ident;
    let wrapper_ident = format_ident!("nu_test_{fn_ident}");
    let static_ident = format_ident!("NU_TEST_{}", fn_ident.to_string().to_shouty_snake_case());

    let wrapper_call = match &item_fn.sig.output {
        ReturnType::Default => quote!(#fn_ident()),
        ReturnType::Type(..) => quote!(#fn_ident()?),
    };
    let wrapper = quote! {
        fn #wrapper_ident() -> ::std::result::Result<(), ::std::boxed::Box<dyn ::std::error::Error>> {
            #wrapper_call;
            ::std::result::Result::Ok(())
        }
    };

    let experimental_options = nu_experimental::ALL.into_iter().map(|option| {
        let field_ident = proc_macro2::Ident::new(
            option.identifier().to_snake_case().as_str(),
            proc_macro2::Span::call_site(),
        );
        quote!(#field_ident: ::std::option::Option::None)
    });

    quote! {
        #wrapper

        #[allow(deprecated, reason = "constructed in macro")]
        #[::nu_test_support::collect_test(nu_test_support::harness::TESTS)]
        #[linkme(crate = ::nu_test_support::harness::linkme)]
        static #static_ident: ::nu_test_support::harness::TestMetadata =
            ::nu_test_support::harness::TestMetadata {
                function: #wrapper_ident,
                name: ::std::sync::LazyLock::new(|| ::std::any::type_name_of_val(&#fn_ident)),
                // TODO: parse these fields
                ignored: (false, ::std::option::Option::None),
                should_panic: (false, ::std::option::Option::None),
                experimental_options: ::nu_test_support::harness::RequestedExperimentalOptions {
                    #(#experimental_options,)*
                }
            };

        #item_fn
    }
}
