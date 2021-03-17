// Helper macros

macro_rules! concat_to_c_str {
    ( $($s:expr),+ ) => ( {
        use std::os::raw::c_char;
        concat!($($s),+, "\0").as_ptr() as *const c_char
    } );
}
