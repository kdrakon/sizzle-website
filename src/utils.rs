use hmac::{Hmac, Mac};
use sha2::Sha256;

use models::Referrer;

type HmacSha256 = Hmac<Sha256>;

const BASE_64_SEPARATOR: char = '🔥';

pub fn base64_encode_referrer(referrer: Referrer) -> String {
    let concat = format!("{}{}{}", referrer.nickname, BASE_64_SEPARATOR, referrer.referrer_code);
    base64::encode(&concat)
}

pub fn base64_decode_referrer(base64str: String) -> Result<Referrer, ()> {
    match base64::decode(&base64str) {
        Err(_) => Err(()),
        Ok(referrer) => {
            match String::from_utf8(referrer) {
                Err(_) => Err(()),
                Ok(referrer) => {
                    let split: Vec<&str> = referrer.split(BASE_64_SEPARATOR).collect::<Vec<&str>>();
                    if split.len() == 2 {
                        Ok(Referrer::new(split[0], split[1]))
                    } else {
                        Err(())
                    }
                }
            }
        }
    }
}

pub fn hmac_bytes(key: &str, nickname: &str, email: &str) -> Vec<u8> {
    let mut mac = HmacSha256::new_varkey(key.as_bytes()).expect("HMAC can take key of any size");
    mac.input(format!("{}{}", nickname, email).as_bytes());
    mac.result().code().to_vec()
}

pub trait AsHexString {
    fn into_hex_string(self) -> String;
}

impl AsHexString for Vec<u8> {
    fn into_hex_string(self) -> String {
        self.into_iter().map(|byte| format!("{:X}", byte)).collect::<Vec<String>>().join("")
    }
}