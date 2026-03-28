/// Appends formatted text to a [`String`].
///
/// Like [`write!`], but does not require handling a [`Result`].
/// Writing to a [`String`] is infallible, so this will not fail.
///
/// # Example
///
/// ```rust
/// # use nu_utils::push_fmt;
/// let mut s = String::new();
/// push_fmt!(s, "Hello {}", "world");
/// push_fmt!(s, ", the answer is {}", 42);
/// assert_eq!(s, "Hello world, the answer is 42");
/// ```
#[macro_export]
macro_rules! push_fmt {
    ($dst:expr, $($arg:tt)*) => {{
        let s: &mut ::std::string::String = &mut $dst;
        ::std::fmt::Write::write_fmt(s, ::std::format_args!($($arg)*))
            .expect("writing to String is infallible");
    }};
}
