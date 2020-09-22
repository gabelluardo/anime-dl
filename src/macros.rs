macro_rules! print_err {
    ($x:expr) => {
        match $x {
            Ok(item) => item,
            Err(err) => return eprintln!("{}", crate::utils::tui::format_err(err)),
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
        $str.replace(crate::utils::PLACEHOLDER, &zfill!($num))
    };
}

#[cfg(feature = "aes")]
macro_rules! delay_for {
    ($time:expr) => {
        tokio::time::delay_for(std::time::Duration::from_millis($time as u64)).await;
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
