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
                    .strip_suffix("::{{closure}}")
                    .unwrap()
            }};
        }

        $crate::location::Location {
            fn_name: fn_name!(),
            file_name: file!(),
            line_no: line!(),
            col_no: column!(),
        }
    }};
}

#[derive(Debug)]
pub struct Location {
    pub fn_name: &'static str,
    pub file_name: &'static str,
    pub line_no: u32,
    pub col_no: u32,
}
