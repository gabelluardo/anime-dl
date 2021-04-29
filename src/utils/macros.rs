macro_rules! bail {
    ($err:expr $(,)?) => {
        return Err($err)
    };
}

macro_rules! gen_url {
    ($str:expr, $num:expr) => {
        $str.replace(crate::utils::PLACEHOLDER, &zfill!($num))
    };
}

macro_rules! eprint {
    ($x:expr) => {
        match $x {
            Ok(item) => item,
            Err(err) => {
                if (err.to_string() != "") {
                    bunt::eprintln!("{$red}{}{/$}", err)
                }
                continue;
            }
        }
    };
}

macro_rules! ok {
    ($x:expr) => {
        match $x {
            Ok(item) => item,
            Err(err) => {
                if (err.to_string() != "") {
                    bunt::eprintln!("{$red}{}{/$}", err);
                }
                return;
            }
        }
    };
}

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
