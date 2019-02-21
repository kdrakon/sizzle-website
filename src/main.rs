#![feature(proc_macro_hygiene, decl_macro)]

extern crate base64;
extern crate clap;
extern crate hmac;
#[macro_use]
extern crate rocket;
#[macro_use]
extern crate rocket_contrib;
#[macro_use]
extern crate serde;
extern crate sha2;

use clap::{App, Arg};
use rocket::fairing::{Fairing, Info};
use rocket::fairing::Kind;
use rocket::http::Status;
use rocket::request::LenientForm;
use rocket::Rocket;
use rocket::State;
use rocket_contrib::databases::rusqlite;
use rocket_contrib::serve::*;
use rocket_contrib::templates::Template;

use models::*;
use utils::*;

mod utils;
mod models;

#[database("sizzle_db")]
struct DatabaseConnection(rusqlite::Connection);

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

    rocket::ignite()
        .mount("/", routes![
            mailchimp_subscribed_get_webhook,
            mailchimp_subscribed_post_webhook,
            refer_a_mate,
            refer_a_mate_link,
            new_refer_a_mate_link
        ])
        .mount("/", StaticFiles::from("static"))
        .attach(Template::fairing())
        .attach(DatabaseConnection::fairing())
        .manage(hmac_config)
        .manage(mailchimp_config)
        .launch();
}

struct DatabaseInit {}

impl Fairing for DatabaseInit {
    fn info(&self) -> Info { Info { name: "DatabaseInit", kind: Kind::Attach } }
    fn on_launch(&self, rocket: &Rocket) {}
}

#[get("/mailchimp/subscribed?<key>", data = "<mailchimp_subscribed_data>")]
fn mailchimp_subscribed_get_webhook(key: String, mailchimp_subscribed_data: LenientForm<MailChimpSubscribeData>,
                                    mailchimp_config: State<MailchimpConfig>, db: DatabaseConnection) -> Status {
    mailchimp_subscribed_post_webhook(key, mailchimp_subscribed_data, mailchimp_config, db)
}

#[post("/mailchimp/subscribed?<key>", data = "<mailchimp_subscribed_data>")]
fn mailchimp_subscribed_post_webhook(key: String, mailchimp_subscribed_data: LenientForm<MailChimpSubscribeData>,
                                     mailchimp_config: State<MailchimpConfig>, db: DatabaseConnection) -> Status {
    match key {
        ref valid_key if mailchimp_config.request_key.eq(valid_key) => {
            match mailchimp_subscribed_data._type.as_str() {
                "subscribe" => {
                    // TODO store referrer entry
                    Status::Accepted
                }
                _ => Status::BadRequest
            }
        }
        _ => Status::Unauthorized
    }
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

