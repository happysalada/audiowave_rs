use cfg_if::cfg_if;
use std::convert::TryFrom;
use std::fmt::Debug;

cfg_if! {
    // https://github.com/rustwasm/console_error_panic_hook#readme
    if #[cfg(feature = "console_error_panic_hook")] {
        extern crate console_error_panic_hook;
        pub use self::console_error_panic_hook::set_once as set_panic_hook;
    } else {
        #[inline]
        pub fn set_panic_hook() {}
    }
}

pub fn get_extension_from_filename(filename: &str) -> Option<&str> {
    use std::ffi::OsStr;
    use std::path::Path;

    Path::new(filename).extension().and_then(OsStr::to_str)
}

// TODO consider not using generics here. It was fun, but it's disgusting
pub fn div_up<T, U>(a: T, b: U) -> usize
where
    <usize as TryFrom<T>>::Error: Debug,
    <usize as TryFrom<U>>::Error: Debug,
    usize: TryFrom<T>,
    usize: TryFrom<U>,
{
    let numerator = usize::try_from(a).unwrap();
    let denominator = usize::try_from(b).unwrap();
    (numerator + (denominator - 1)) / denominator
}
