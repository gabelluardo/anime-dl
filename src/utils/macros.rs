/// Throw the error to the caller function
// macro_rules! bail {
//     ($err:expr $(,)?) => {
//         return Err($err)
//     };
// }

/// Fill url placeholder with episode digit
macro_rules! gen_url {
    ($str:expr, $num:expr, $alignment:expr) => {
        $str.replace("_{}", &zfill!($num, $alignment))
    };
}

/// Print the error message and continue
macro_rules! unroll {
    ($x:expr) => {
        match $x {
            Ok(item) => item,
            Err(err) => {
                // if (!matches!(err, Err("..."))) {
                eprintln!("{}", err.red());
                // }
                continue;
            }
        }
    };
}

/// Print the error message and return
macro_rules! ok {
    ($x:expr) => {
        match $x {
            Ok(item) => item,
            Err(err) => {
                // if (!matches!(err, "...")) {
                eprintln!("{}", err.red());
                // }
                return;
            }
        }
    };
}

/// Format digit with 2 zero into a string
macro_rules! zfill {
    ($num:expr, $alignment:expr) => {
        format!("_{:0fill$}", $num, fill = $alignment)
    };
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_zfill() {
        assert_eq!(zfill!(1, 2), "_01");
        assert_eq!(zfill!(200, 2), "_200");
        assert_eq!(zfill!(15, 3), "_015")
    }

    #[test]
    fn test_gen_url() {
        let url = "https://robe_{}_.tld";

        assert_eq!(gen_url!(url, 1, 2), "https://robe_01_.tld");
        assert_eq!(gen_url!(url, 14, 2), "https://robe_14_.tld");
        assert_eq!(gen_url!(url, 1400, 2), "https://robe_1400_.tld");

        assert_eq!(gen_url!(url, 1, 3), "https://robe_001_.tld");
        assert_eq!(gen_url!(url, 14, 3), "https://robe_014_.tld");
        assert_eq!(gen_url!(url, 1400, 3), "https://robe_1400_.tld");
    }
}
