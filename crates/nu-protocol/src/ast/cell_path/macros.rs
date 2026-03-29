use crate::{ast::PathMember, casing::Casing, Span};

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
    (($val:literal)) => { $crate::ast::cell_path::macros::TestPathMember::from($val).build() };
    (($val:literal)?) => { $crate::ast::cell_path::macros::TestPathMember::from($val).optional() };
    (($val:literal)!) => { $crate::ast::cell_path::macros::TestPathMember::from($val).insensitive() };
    (($val:literal)?!) => { $crate::ast::cell_path::macros::TestPathMember::from($val).optional_and_insensitive() };
    (($val:literal)!?) => { $crate::ast::cell_path::macros::TestPathMember::from($val).optional_and_insensitive() };
}

#[doc(hidden)]
#[macro_export]
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

/// Build a [`CellPath`](super::CellPath) for tests from a dot-separated token stream.
///
/// This macro expands to a `CellPath { members: Vec<PathMember> }` using
/// [`Span::test_data()`] for all members, making it convenient for unit tests.
///
/// Accepted segments:
/// - Identifiers, which become string members via `stringify!`.
/// - String or integer literals.
/// - Parenthesized identifiers to use a variable's value (e.g. `(name)`).
///
/// # Examples
///
/// ```
/// # #[macro_use]
/// # extern crate nu_protocol;
/// use nu_protocol::test_cell_path;
///
/// let path = test_cell_path!("a b c".col!?.2?);
/// assert_eq!(path.to_string(), r#"$."a b c".col!?.2?"#);
/// ```
#[macro_export]
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
    #[rustfmt::skip]
    fn test_cell_path_works() {
        let name = "col";
        let index = 3;
        assert_eq!(test_cell_path!(1.abc).to_string(), "$.1.abc");
        assert_eq!(test_cell_path!((2).something).to_string(), "$.2.something");
        assert_eq!(test_cell_path!((2)?.something).to_string(), "$.2?.something");
        assert_eq!(test_cell_path!(abc.4).to_string(), "$.abc.4");
        assert_eq!(test_cell_path!(abc).to_string(), "$.abc");
        assert_eq!(test_cell_path!(abc.def.ghi).to_string(), "$.abc.def.ghi");
        assert_eq!(test_cell_path!(abc?.def).to_string(), "$.abc?.def");
        assert_eq!(test_cell_path!(abc!.def).to_string(), "$.abc!.def");
        assert_eq!(test_cell_path!(abc!?.def!).to_string(), "$.abc!?.def!");
        assert_eq!(test_cell_path!("a b c").to_string(), r#"$."a b c""#);
        assert_eq!(test_cell_path!("a b c"."d e f").to_string(), r#"$."a b c"."d e f""#);
        assert_eq!(test_cell_path!("a b c".col!?.2?).to_string(), r#"$."a b c".col!?.2?"#);
        assert_eq!(test_cell_path!("spaced"?!.value).to_string(), r#"$.spaced!?.value"#);
        assert_eq!(test_cell_path!((name).value).to_string(), "$.col.value");
        assert_eq!(test_cell_path!((index)?.value).to_string(), "$.3?.value");
    }
}
