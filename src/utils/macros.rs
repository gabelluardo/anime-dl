/// Throw the error to the caller function
macro_rules! bail {
    ($err:expr $(,)?) => {
        return Err($err)
    };
}

/// Fill url placeholder with episode digit
macro_rules! gen_url {
    ($str:expr, $num:expr) => {
        $str.replace("_{}", &zfill!($num))
    };
}

/// Print the error message and continue
macro_rules! unroll {
    ($x:expr) => {
        match $x {
            Ok(item) => item,
            Err(err) => {
                if (!matches!(err, Error::Quit)) {
                    bunt::eprintln!("{$red}{}{/$}", err)
                }
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
                if (!matches!(err, Error::Quit)) {
                    bunt::eprintln!("{$red}{}{/$}", err);
                }
                return;
            }
        }
    };
}

/// Format digit with 2 zero into a string
macro_rules! zfill {
    ($num:expr) => {
        format!("_{:02}", $num)
    };
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_zfill() {
        assert_eq!(zfill!(1), "_01");
        assert_eq!(zfill!(200), "_200");
    }

    #[test]
    fn test_gen_url() {
        let url = "http://robe_{}_.tld";

        assert_eq!(gen_url!(url, 1), "http://robe_01_.tld");
        assert_eq!(gen_url!(url, 14), "http://robe_14_.tld");
        assert_eq!(gen_url!(url, 1400), "http://robe_1400_.tld");
    }
}
