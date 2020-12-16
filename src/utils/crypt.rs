use super::*;

use aes_soft::Aes128;
use block_modes::{block_padding, BlockMode, Cbc};

pub fn extract_hex(text: &str, matcher: &str) -> Result<Vec<Vec<u8>>> {
    let re = Regex::new(matcher).unwrap();
    let cap = re
        .captures_iter(&text)
        .map(|c| (&c[0] as &str).to_string())
        .map(|s| {
            s.trim_matches(|c| c == '(' || c == ')' || c == '"')
                .to_string()
        })
        .collect::<Vec<_>>();

    let mut res = vec![];
    for s in cap {
        res.push(hex::decode(s)?);
    }

    Ok(res)
}

pub fn encode(key: &[u8], iv: &[u8], data: &[u8]) -> Result<String> {
    type Aes128Cbc = Cbc<Aes128, block_padding::NoPadding>;

    let cipher = Aes128Cbc::new_var(&key, &iv)?;
    let out = hex::encode(cipher.decrypt_vec(&data)?);

    Ok(out)
}
