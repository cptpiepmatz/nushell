use std::mem;

use heck::ToShoutySnakeCase;
use quote::{format_ident, quote};
use syn::{
    Attribute, Expr, Ident, ItemFn, Lit, LitBool, LitStr, Meta, MetaNameValue, PatLit, Path,
    ReturnType, Token,
    parse::{ParseStream, Parser},
};

pub fn test(mut item_fn: ItemFn) -> proc_macro2::TokenStream {
    let attrs = match TestAttributes::try_from(mem::take(&mut item_fn.attrs)) {
        Ok(attrs) => attrs,
        Err(err) => return err.to_compile_error(),
    };
    let attr_rest = attrs.rest;

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

    let ignored = match attrs.ignore {
        (value, None) => quote!((#value, ::std::option::Option::None)),
        (value, Some(msg)) => quote!((#value, ::std::option::Option::Some(#msg))),
    };

    let should_panic = match attrs.should_panic {
        (value, None) => quote!((#value, ::std::option::Option::None)),
        (value, Some(msg)) => quote!((#value, ::std::option::Option::Some(#msg))),
    };

    quote! {
        #wrapper

        #[allow(deprecated, reason = "constructed in macro")]
        #[::nu_test_support::collect_test(nu_test_support::harness::TESTS)]
        #[linkme(crate = ::nu_test_support::harness::linkme)]
        static #static_ident: ::nu_test_support::harness::TestMetadata =
            ::nu_test_support::harness::TestMetadata {
                function: #wrapper_ident,
                name: ::std::sync::LazyLock::new(|| ::std::any::type_name_of_val(&#fn_ident)),
                ignored: #ignored,
                should_panic: #should_panic,
                // TODO: parse these fields
                experimental_options: &[],
                environment_variables: &[],
            };

        #(#attr_rest)*
        #item_fn
    }
}

#[derive(Debug, Default)]
pub struct TestAttributes {
    pub ignore: (bool, Option<LitStr>),
    pub should_panic: (bool, Option<LitStr>),
    pub experimental_options: Vec<(Path, LitBool)>,
    pub environment_variables: Vec<(Ident, LitStr)>,
    pub rest: Vec<Attribute>,
}

impl TryFrom<Vec<Attribute>> for TestAttributes {
    type Error = syn::Error;

    fn try_from(attrs: Vec<Attribute>) -> Result<Self, Self::Error> {
        let mut test_attrs = TestAttributes::default();
        for attr in attrs {
            let Some(ident) = attr.path().get_ident() else {
                test_attrs.rest.push(attr);
                continue;
            };

            match ident.to_string().as_str() {
                "ignore" => match attr.meta {
                    Meta::Path(_) => test_attrs.ignore.0 = true,
                    Meta::NameValue(MetaNameValue { value, .. }) => match value {
                        Expr::Lit(lit) => match lit.lit {
                            Lit::Str(lit_str) => {
                                test_attrs.ignore.0 = true;
                                test_attrs.ignore.1 = Some(lit_str);
                            }
                            _ => todo!("error"),
                        },
                        _ => todo!("error"),
                    },
                    Meta::List(meta_list) => todo!("error"),
                },

                "should_panic" => match attr.meta {
                    Meta::Path(_) => test_attrs.should_panic.0 = true,
                    Meta::List(meta_list) => meta_list.parse_nested_meta(|meta| {
                        if meta.path.is_ident("expected") {
                            let value = meta.value()?;
                            let expected: LitStr = value.parse()?;
                            test_attrs.should_panic.0 = true;
                            test_attrs.should_panic.1 = Some(expected);
                            Ok(())
                        } else {
                            todo!("error")
                        }
                    })?,
                    Meta::NameValue(_) => todo!("error"),
                },

                "experimental_option" => todo!(),
                "env" => todo!(),
                _ => test_attrs.rest.push(attr),
            }
        }

        Ok(test_attrs)
    }
}
