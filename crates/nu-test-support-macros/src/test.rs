use std::mem;

use heck::ToShoutySnakeCase;
use quote::{format_ident, quote};
use syn::{
    Attribute, Expr, Ident, ItemFn, Lit, LitBool, LitStr, Meta, MetaNameValue, Path, ReturnType,
    Token, parse::ParseStream,
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

    let experimental_options = attrs.experimental_options.into_iter().map(|(path, lit)| {
        let lit = lit.map(|lit| lit.value).unwrap_or(true);
        quote!((&#path, #lit))
    });

    let environment_variables = attrs.environment_variables.into_iter().map(|(key, value)| {
        let key = key.to_string();
        quote!((#key, #value))
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
                ignored: #ignored,
                should_panic: #should_panic,
                experimental_options: &[#(#experimental_options),*],
                environment_variables: &[#(#environment_variables),*],
            };

        #(#attr_rest)*
        #item_fn
    }
}

#[derive(Default)]
pub struct TestAttributes {
    pub ignore: (bool, Option<LitStr>),
    pub should_panic: (bool, Option<LitStr>),
    pub experimental_options: Vec<(Path, Option<LitBool>)>,
    pub environment_variables: Vec<(Ident, Expr)>,
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
                    Meta::List(_meta_list) => todo!("error"),
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

                "experimental_options" => {
                    fn parse(input: ParseStream) -> syn::Result<Vec<(Path, Option<LitBool>)>> {
                        Ok(input
                            .parse_terminated(
                                |input| {
                                    let path: Path = input.parse()?;
                                    if !input.peek(Token![=]) {
                                        return Ok((path, None));
                                    };
                                    let _: Token![=] = input.parse()?;
                                    let value: LitBool = input.parse()?;
                                    Ok((path, Some(value)))
                                },
                                Token![,],
                            )?
                            .into_iter()
                            .collect())
                    }

                    let options = attr.parse_args_with(parse)?;
                    test_attrs.experimental_options.extend(options);
                }

                "env" => {
                    fn parse(input: ParseStream) -> syn::Result<Vec<(Ident, Expr)>> {
                        Ok(input
                            .parse_terminated(
                                |input| {
                                    let key: Ident = input.parse()?;
                                    let _: Token![=] = input.parse()?;
                                    let value: Expr = input.parse()?;
                                    Ok((key, value))
                                },
                                Token![,],
                            )?
                            .into_iter()
                            .collect())
                    }

                    let envs = attr.parse_args_with(parse)?;
                    test_attrs.environment_variables.extend(envs);
                }

                _ => test_attrs.rest.push(attr),
            }
        }

        Ok(test_attrs)
    }
}
