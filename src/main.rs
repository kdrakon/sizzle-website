#![feature(proc_macro_hygiene, decl_macro)]

extern crate base64;
extern crate chrono;
extern crate clap;
extern crate hmac;
#[macro_use]
extern crate rocket;
#[macro_use]
extern crate rocket_contrib;
#[macro_use]
extern crate serde;
extern crate sha2;

use chrono::prelude::*;
use clap::{App, Arg};
use rocket::{Rocket, State};
use rocket::fairing::{Fairing, Info, Kind};
use rocket::http::Status;
use rocket::request::LenientForm;
use rocket_contrib::databases::rusqlite;
use rocket_contrib::serve::*;
use rocket_contrib::templates::Template;
use rusqlite::{Connection, Row, Statement};

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
                     let connection = db.0;

                    // TODO need to have this done once in some kind of migration stage
                    connection.execute(
                        "CREATE TABLE IF NOT EXISTS referred_subscriptions (
                          id                   TEXT PRIMARY KEY,
                          referrer_code        TEXT NOT NULL,
                          referrer_nickname    TEXT NOT NULL,
                          fired_at             DATETIME
                          )",
                        &[],
                    ).unwrap();

                    let fired_at: DateTime<Utc> = Utc.datetime_from_str(&mailchimp_subscribed_data.fired_at, "%Y-%m-%d %H:%M:%S").unwrap();

                    match
                        connection.execute(
                            "INSERT INTO referred_subscriptions (id, referrer_code, referrer_nickname, fired_at)
                                  VALUES (?1, ?2, ?3, ?4)",
                            &[&mailchimp_subscribed_data.id, &mailchimp_subscribed_data.referrer_code, &mailchimp_subscribed_data.referrer_nickname, &fired_at.timestamp()],
                        ) {
                        Ok(_) => Status::Accepted,
                        Err(_) => Status::InternalServerError
                    }
                }
                _ => Status::BadRequest
            }
        }
        _ => Status::Unauthorized
    }
}

#[get("/refer-a-mate")]
fn refer_a_mate(db: DatabaseConnection) -> Template {
    let context: ReferAMateContext = ReferAMateContext {
        top_referrers: top_referrers(db).unwrap_or(TopReferrers::empty()),
        referrer_context: None,
    };
    Template::render("refer-a-mate", context)
}

#[get("/refer-a-mate?<referrer>")]
fn refer_a_mate_link(referrer: String, db: DatabaseConnection) -> Template {
    let referrer_base64_link = referrer.clone();
    let referrer = base64_decode_referrer(referrer).ok();
    let context: ReferAMateContext = ReferAMateContext {
        top_referrers: top_referrers(db).unwrap_or(TopReferrers::empty()),
        referrer_context: referrer.map(|referrer| ReferrerContext { referrer, referrer_base64_link, show_link: false }),
    };
    Template::render("refer-a-mate", context)
}

#[post("/refer-a-mate", data = "<form_data>")]
fn new_refer_a_mate_link(form_data: LenientForm<ReferAMateFormData>, hmac_config: State<HmacConfig>, db: DatabaseConnection) -> Template {
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
        top_referrers: top_referrers(db).unwrap_or(TopReferrers::empty()),
        referrer_context: Some(referrer_context),
    };
    Template::render("refer-a-mate", context)
}

fn top_referrers(db: DatabaseConnection) -> Result<TopReferrers, rusqlite::Error> {
    fn query(connection: &Connection) -> Result<Statement, rusqlite::Error> {
        connection.prepare("
            SELECT
               referrer_nickname,
               referrer_code,
               strftime('%Y-%m', datetime(fired_at, 'unixepoch', 'localtime')) AS year_month,
               count(*) as subscribed
            FROM referred_subscriptions
            WHERE year_month = ?1
            GROUP BY referrer_code, year_month
            ORDER BY subscribed DESC, min(fired_at) ASC
            LIMIT 10"
        )
    }

    fn map_referrer(row: &Row) -> Referrer {
        let nickname: String = row.get(0);
        let referrer_code: String = row.get(1);
        Referrer::new(nickname.as_str(), referrer_code.as_str())
    }

    fn get_referrers(connection: &Connection, year_month: (i32, i8)) -> Result<Vec<Referrer>, rusqlite::Error> {
        let year_month = format!("{:04}-{:02}", year_month.0, year_month.1);
        query(connection).and_then(|mut statement| {
            statement.query_map(&[&year_month], map_referrer)
                .and_then(|results| results.collect::<Result<Vec<Referrer>, rusqlite::Error>>())
        })
    }

    let now = Local::now();
    let this_year_month = (now.year(), now.month() as i8);
    let last_year_month = match this_year_month {
        (year, month) if month == 1 => (year - 1, 12),
        (year, month) => (year, month - 1)
    };

    get_referrers(&db.0, this_year_month)
        .and_then(|this_month| {
            get_referrers(&db.0, last_year_month).map(|last_month| {
                TopReferrers { this_month, last_month }
            })
        })
}

