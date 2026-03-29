use crate::{Span, ast::PathMember, casing::Casing};

#[doc(hidden)]
pub struct TestPathMember<From>(From);

impl<S: Into<String>> From<S> for TestPathMember<String> {
    fn from(value: S) -> Self {
        Self(value.into())
    }
}

impl From<usize> for TestPathMember<usize> {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

impl TestPathMember<String> {
    pub fn build(self) -> PathMember {
        PathMember::String {
            val: self.0,
            span: Span::test_data(),
            optional: false,
            casing: Casing::Sensitive,
        }
    }

    pub fn optional(self) -> PathMember {
        PathMember::String {
            val: self.0,
            span: Span::test_data(),
            optional: true,
            casing: Casing::Sensitive,
        }
    }

    pub fn insensitive(self) -> PathMember {
        PathMember::String {
            val: self.0,
            span: Span::test_data(),
            optional: false,
            casing: Casing::Insensitive,
        }
    }

    pub fn optional_and_insensitive(self) -> PathMember {
        PathMember::String {
            val: self.0,
            span: Span::test_data(),
            optional: true,
            casing: Casing::Insensitive,
        }
    }
}

impl TestPathMember<usize> {
    pub fn build(self) -> PathMember {
        PathMember::Int {
            val: self.0,
            span: Span::test_data(),
            optional: false,
        }
    }

    pub fn optional(self) -> PathMember {
        PathMember::Int {
            val: self.0,
            span: Span::test_data(),
            optional: true,
        }
    }
}

#[doc(hidden)]
#[rustfmt::skip]
#[macro_export]
macro_rules! test_path_member {
    ($val:literal) => { $crate::ast::cell_path::macros::TestPathMember::from($val).build() };
    ($val:literal?) => { $crate::ast::cell_path::macros::TestPathMember::from($val).optional() };
    ($val:literal!) => { $crate::ast::cell_path::macros::TestPathMember::from($val).insensitive() };
    ($val:literal?!) => { $crate::ast::cell_path::macros::TestPathMember::from($val).optional_and_insensitive() };
    ($val:literal!?) => { $crate::ast::cell_path::macros::TestPathMember::from($val).optional_and_insensitive() };
    ($val:ident) => { $crate::ast::cell_path::macros::TestPathMember::from(stringify!($val)).build() };
    ($val:ident?) => { $crate::ast::cell_path::macros::TestPathMember::from(stringify!($val)).optional() };
    ($val:ident!) => { $crate::ast::cell_path::macros::TestPathMember::from(stringify!($val)).insensitive() };
    ($val:ident?!) => { $crate::ast::cell_path::macros::TestPathMember::from(stringify!($val)).optional_and_insensitive() };
    ($val:ident!?) => { $crate::ast::cell_path::macros::TestPathMember::from(stringify!($val)).optional_and_insensitive() };
    (($val:ident)) => { $crate::ast::cell_path::macros::TestPathMember::from($val).build() };
    (($val:ident)?) => { $crate::ast::cell_path::macros::TestPathMember::from($val).optional() };
    (($val:ident)!) => { $crate::ast::cell_path::macros::TestPathMember::from($val).insensitive() };
    (($val:ident)?!) => { $crate::ast::cell_path::macros::TestPathMember::from($val).optional_and_insensitive() };
    (($val:ident)!?) => { $crate::ast::cell_path::macros::TestPathMember::from($val).optional_and_insensitive() };
}

#[doc(hidden)]
#[macro_export(local_inner_macros)]
macro_rules! test_path_members {
    // hit dot, finalize current segment
    ([$($out:expr,)*] [$($cur:tt)+] . $($rest:tt)*) => {
        test_path_members!(
            [$($out,)* test_path_member!($($cur)+),]
            []
            $($rest)*
        )
    };

    // end of input, finalize last segment
    ([$($out:expr,)*] [$($cur:tt)+]) => {
        ::std::vec![
            $($out,)*
            test_path_member!($($cur)+)
        ]
    };

    // keep munching tokens into current segment
    ([$($out:expr,)*] [$($cur:tt)*] $next:tt $($rest:tt)*) => {
        test_path_members!(
            [$($out,)*]
            [$($cur)* $next]
            $($rest)*
        )
    };
}

#[macro_export(local_inner_macros)]
macro_rules! test_cell_path {
    ($($input:tt)*) => {{
        $crate::ast::CellPath {
            members: test_path_members!([] [] $($input)*)
        }
    }};
}

#[cfg(test)]
mod tests {
    #[test]
    fn compile_single_path_member() {
        let s = "abc";
        let n = "3";
        let _ = [
            test_path_member!(abc),
            test_path_member!(abc?),
            test_path_member!(abc!),
            test_path_member!(abc?!),
            test_path_member!(abc!?),
            test_path_member!("abc"),
            test_path_member!("abc"?),
            test_path_member!("abc"!),
            test_path_member!("abc"?!),
            test_path_member!("abc"!?),
            test_path_member!(1),
            test_path_member!(1?),
            test_path_member!((s)),
            test_path_member!((s)!),
            test_path_member!((s)?),
            test_path_member!((s)!?),
            test_path_member!((s)?!),
            test_path_member!((n)),
            test_path_member!((n)?),
        ];
    }

    #[test]
    fn test_cell_path_works() {
        assert_eq!(test_cell_path!(1.abc).to_string(), "$.1.abc");
        assert_eq!(test_cell_path!(abc.4).to_string(), "$.abc.4");
        assert_eq!(test_cell_path!(abc).to_string(), "$.abc");
        assert_eq!(test_cell_path!("a b c").to_string(), r#"$."a b c""#);
    }
}
