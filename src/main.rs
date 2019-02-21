#![feature(proc_macro_hygiene, decl_macro)]

extern crate base64;
extern crate clap;
extern crate hmac;
#[macro_use]
extern crate rocket;
extern crate rocket_contrib;
#[macro_use]
extern crate serde;
extern crate sha2;

use std::env;

use clap::{App, Arg};
use hmac::{Hmac, Mac};
use rocket::http::Status;
use rocket::request::LenientForm;
use rocket::State;
use rocket_contrib::serve::*;
use rocket_contrib::templates::Template;
use sha2::Sha256;

const BASE_64_SEPARATOR: char = '🔥';

type HmacSha256 = Hmac<Sha256>;

struct HmacConfig {
    secret_key: String
}

struct MailchimpConfig {
    request_key: String
}

fn main() {
    let matches = App::new("sizzle-website")
        .arg(
            Arg::with_name("hmac-secret-key")
                .short("h")
                .required(true)
                .takes_value(true)
                .help("The secret key used for HMAC"),
        )
        .arg(
            Arg::with_name("mailchimp-request-key")
                .short("m")
                .required(true)
                .takes_value(true)
                .help("The request key that the Mailchimp Webhook must supply"),
        )
        .get_matches();

    let hmac_config = HmacConfig { secret_key: String::from(matches.value_of("hmac-secret-key").unwrap()) };
    let mailchimp_config = MailchimpConfig { request_key: String::from(matches.value_of("mailchimp-request-key").unwrap()) };

    rocket
    ::ignite()
        .mount("/", routes![
            mailchimp_subscribed_get_webhook,
            mailchimp_subscribed_post_webhook,
            refer_a_mate,
            refer_a_mate_link,
            new_refer_a_mate_link
        ])
        .mount("/", StaticFiles::from("static"))
        .attach(Template::fairing())
        .manage(hmac_config)
        .manage(mailchimp_config)
        .launch();
}

#[derive(FromForm, Debug)]
struct MailChimpSubscribeData {
    #[form(field = "type")]
    _type: String,
    #[form(field = "data%5Bemail%5D")]
    email: String,
    #[form(field = "data%5Bmerges%5D%5BREFERRER_CODE%5D")]
    referrer_code: String,
    #[form(field = "data%5Bmerges%5D%5BREFERRER_NICKNAME%5D")]
    referrer_nickname: String,
}

// TODO need secret query key that only mailchimp will know
#[get("/mailchimp/subscribed", data = "<mailchimp_subscribed_data>")]
fn mailchimp_subscribed_get_webhook(mailchimp_subscribed_data: LenientForm<MailChimpSubscribeData>, mailchimp_config: State<MailchimpConfig>) -> Result<Status, Status> {
    mailchimp_subscribed_post_webhook(mailchimp_subscribed_data, mailchimp_config)
}

#[post("/mailchimp/subscribed", data = "<mailchimp_subscribed_data>")]
fn mailchimp_subscribed_post_webhook(mailchimp_subscribed_data: LenientForm<MailChimpSubscribeData>, mailchimp_config: State<MailchimpConfig>) -> Result<Status, Status> {
    // TODO store referrer entry
    Ok(Status::Accepted)
}

#[derive(Serialize)]
struct ReferAMateContext {
    top_referrers: TopReferrers,
    referrer_context: Option<ReferrerContext>,
}

#[derive(Serialize, Clone, Debug)]
struct Referrer {
    nickname: String,
    referrer_code: String,
}

impl Referrer {
    fn new(nickname: &str, referrer_code: &str) -> Referrer {
        let nickname = String::from(nickname);
        let referrer_code = String::from(referrer_code);
        Referrer { nickname, referrer_code }
    }
}

#[derive(Serialize, Debug)]
struct ReferrerContext {
    referrer: Referrer,
    referrer_base64_link: String,
    show_link: bool,
}

#[derive(Serialize)]
struct TopReferrers {
    this_month: Vec<Referrer>,
    last_month: Vec<Referrer>,
}

#[get("/refer-a-mate")]
fn refer_a_mate() -> Template {
    let context: ReferAMateContext = ReferAMateContext {
        top_referrers: top_referrers(),
        referrer_context: None,
    };
    Template::render("refer-a-mate", context)
}

#[get("/refer-a-mate?<referrer>")]
fn refer_a_mate_link(referrer: String) -> Template {
    let referrer_base64_link = referrer.clone();
    let referrer = base64_decode_referrer(referrer).ok();
    let context: ReferAMateContext = ReferAMateContext {
        top_referrers: top_referrers(),
        referrer_context: referrer.map(|referrer| ReferrerContext { referrer, referrer_base64_link, show_link: false }),
    };
    Template::render("refer-a-mate", context)
}

#[derive(FromForm)]
struct ReferAMateFormData {
    email: String,
    nickname: String,
}

#[post("/refer-a-mate", data = "<form_data>")]
fn new_refer_a_mate_link(form_data: LenientForm<ReferAMateFormData>, hmac_config: State<HmacConfig>) -> Template {
    let referrer_code =
        hmac_bytes(hmac_config.secret_key.as_str(), form_data.nickname.as_str(), form_data.email.as_str()).into_hex_string();
    let referrer = Referrer::new(form_data.nickname.as_str(), referrer_code.as_str());
    let referrer_context =
        ReferrerContext {
            referrer: referrer.clone(),
            referrer_base64_link: base64_encode_referrer(referrer),
            show_link: true,
        };

    let context: ReferAMateContext = ReferAMateContext {
        top_referrers: top_referrers(),
        referrer_context: Some(referrer_context),
    };
    Template::render("refer-a-mate", context)
}

fn top_referrers() -> TopReferrers {
    // TODO DB query
    TopReferrers {
        this_month: vec![
            Referrer::new("Sean", "1234"),
            Referrer::new("Jemma", "1234"),
        ],
        last_month: vec![Referrer::new("Grayson", "1234")],
    }
}

fn base64_encode_referrer(referrer: Referrer) -> String {
    let concat = format!("{}{}{}", referrer.nickname, BASE_64_SEPARATOR, referrer.referrer_code);
    base64::encode(&concat)
}

fn base64_decode_referrer(base64str: String) -> Result<Referrer, ()> {
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

fn hmac_bytes(key: &str, nickname: &str, email: &str) -> Vec<u8> {
    let mut mac = HmacSha256::new_varkey(key.as_bytes()).expect("HMAC can take key of any size");
    mac.input(format!("{}{}", nickname, email).as_bytes());
    mac.result().code().to_vec()
}

trait AsHexString {
    fn into_hex_string(self) -> String;
}

impl AsHexString for Vec<u8> {
    fn into_hex_string(self) -> String {
        self.into_iter().map(|byte| format!("{:X}", byte)).collect::<Vec<String>>().join("")
    }
}