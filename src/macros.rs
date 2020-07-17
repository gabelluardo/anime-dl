macro_rules! print_err {
    ($x:expr) => {
        match $x {
            Ok(item) => item,
            Err(err) => return eprintln!("{}", $crate::utils::format_err(err)),
        }
    };
}

macro_rules! zfill {
    ($num:expr) => {
        format!("_{:02}", $num)
    };
}

macro_rules! gen_url {
    ($str:expr, $num:expr) => {
        $str.replace($crate::utils::REGEX_VALUE, &zfill!($num))
    };
}
