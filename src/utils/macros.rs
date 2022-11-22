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
                if !err.is::<crate::errors::Quit>() {
                    eprintln!("{}", err.red());
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
                if !err.is::<crate::errors::Quit>() {
                    eprintln!("{}", err.red());
                }
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

/// Convert a string from CamelCase to Title Case
macro_rules! to_title_case {
    ($str:expr) => {
        crate::utils::recase_string(&$str, ' ', false)
    };
}

/// Convert a string from CamelCase to snake_case
macro_rules! to_snake_case {
    ($str:expr) => {
        crate::utils::recase_string(&$str, '_', true)
    };
}

#[cfg(test)]
mod tests {
    // use anyhow::{anyhow, bail, Error};
    // use owo_colors::OwoColorize;
    //
    // #[test]
    // fn test_() {
    //     let x = || bail!(crate::errors::QuitError::Quit);
    //
    //     match x() {
    //         Ok(item) => item,
    //         Err(err) => {
    //             if !err.is::<crate::errors::QuitError>() {
    //                 eprintln!("{}", err.red());
    //             }
    //             return;
    //         }
    //     }
    // }

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

    #[test]
    fn test_to_title_case() {
        let s = "StringaInTitleCase-con-delle-linee";
        assert_eq!(to_title_case!(s), "Stringa In Title Case-con-delle-linee");

        let s = "StringaCoNMaiuscole";
        assert_eq!(to_title_case!(s), "Stringa CoN Maiuscole");

        let s = "HighSchoolDxD";
        assert_eq!(to_title_case!(s), "High School DxD");

        let s = "IDInvaded";
        assert_eq!(to_title_case!(s), "ID Invaded");

        let s = "SwordArtOnline2";
        assert_eq!(to_title_case!(s), "Sword Art Online 2");

        let s = "SAO2";
        assert_eq!(to_title_case!(s), "SAO 2");

        let s = "SlimeTaoshite300-nen";
        assert_eq!(to_title_case!(s), "Slime Taoshite 300-nen");

        let s = "HigeWoSoruSoshiteJoshikouseiWoHirou";
        assert_eq!(
            to_title_case!(s),
            "Hige Wo Soru Soshite Joshikousei Wo Hirou"
        )
    }

    #[test]
    fn test_to_snake_case() {
        let s = "StringaInTitleCase-con-delle-linee";
        assert_eq!(to_snake_case!(s), "stringa_in_title_case-con-delle-linee");

        let s = "StringaCoNMaiuscole";
        assert_eq!(to_snake_case!(s), "stringa_con_maiuscole");

        let s = "HighSchoolDxD";
        assert_eq!(to_snake_case!(s), "high_school_dxd");

        let s = "IDInvaded";
        assert_eq!(to_snake_case!(s), "id_invaded");

        let s = "SwordArtOnline2";
        assert_eq!(to_snake_case!(s), "sword_art_online_2");

        let s = "SAO2";
        assert_eq!(to_snake_case!(s), "sao_2");

        let s = "SlimeTaoshite300-nen";
        assert_eq!(to_snake_case!(s), "slime_taoshite_300-nen");

        let s = "HigeWoSoruSoshiteJoshikouseiWoHirou";
        assert_eq!(
            to_snake_case!(s),
            "hige_wo_soru_soshite_joshikousei_wo_hirou"
        )
    }
}
