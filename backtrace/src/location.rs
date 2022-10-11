/// Produces a [`Location`] when invoked in a function body.
///
/// ```
/// use async_backtrace::{location, Location};
///
/// #[tokio::main]
/// async fn main() {
///     assert_eq!(location!(), Location {
///         fn_name: "rust_out::main::{{closure}}",
///         file_name: "src/location.rs",
///         line_no: 7,
///         col_no: 16,
///     });
///
///     async {
///         assert_eq!(location!(), Location {
///             fn_name: "rust_out::main::{{closure}}::{{closure}}",
///             file_name: "src/location.rs",
///             line_no: 15,
///             col_no: 20,
///         });
///     }.await;
///     
///     (|| async {
///         assert_eq!(location!(), Location {
///             fn_name: "rust_out::main::{{closure}}::{{closure}}::{{closure}}",
///             file_name: "src/location.rs",
///             line_no: 24,
///             col_no: 20,
///         });
///     })().await;
/// }
/// ```
#[macro_export]
macro_rules! location {
    () => {{
        macro_rules! fn_name {
            () => {{
                async {}.await;
                fn type_name_of_val<T: ?Sized>(_: &T) -> &'static str {
                    core::any::type_name::<T>()
                }
                type_name_of_val(&|| {})
                    .strip_suffix("::{{closure}}")
                    .unwrap()
            }};
        }

        $crate::Location {
            fn_name: fn_name!(),
            file_name: file!(),
            line_no: line!(),
            col_no: column!(),
        }
    }};
}

/// A source code location in a function body.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Location {
    /// The name of the surrounding function.
    pub fn_name: &'static str,
    /// The name of the file in which this location occurs.
    pub file_name: &'static str,
    /// The line number of this location.
    pub line_no: u32,
    /// The column number of this location.
    pub col_no: u32,
}
