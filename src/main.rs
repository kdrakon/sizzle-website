#![feature(proc_macro_hygiene, decl_macro)]

extern crate base64;
#[macro_use]
extern crate rocket;
extern crate rocket_contrib;
#[macro_use]
extern crate serde;

use rocket::http::Status;
use rocket::http::uri::Uri;
use rocket::request::LenientForm;
use rocket_contrib::serve::*;
use rocket_contrib::templates::Template;
use serde::ser::Serialize;

fn main() {
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
    #[form(field = "data%5Bmerges%5D%5BREFERRER_EMAIL_CODE%5D")]
    referrer_email_code: String,
    #[form(field = "data%5Bmerges%5D%5BREFERRER_NICKNAME%5D")]
    referrer_nickname: String,
}

// TODO need secret query key that only mailchimp will know
#[get("/mailchimp/subscribed", data = "<mailchimp_subscribed_data>")]
fn mailchimp_subscribed_get_webhook(mailchimp_subscribed_data: LenientForm<MailChimpSubscribeData>) -> Result<Status, Status> {
    mailchimp_subscribed_post_webhook(mailchimp_subscribed_data)
}

#[post("/mailchimp/subscribed", data = "<mailchimp_subscribed_data>")]
fn mailchimp_subscribed_post_webhook(mailchimp_subscribed_data: LenientForm<MailChimpSubscribeData>) -> Result<Status, Status> {
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
    referrer_email_code: String,
}

impl Referrer {
    fn new(nickname: &str, referrer_code: &str, referrer_email_code: &str) -> Referrer {
        let nickname = String::from(nickname);
        let referrer_code = String::from(referrer_code);
        let referrer_email_code = String::from(referrer_email_code);
        Referrer { nickname, referrer_code, referrer_email_code }
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
    let referrer_base64_link = Uri::percent_encode(referrer.as_str()).into_owned();
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
fn new_refer_a_mate_link(form_data: LenientForm<ReferAMateFormData>) -> Template {
    // TODO generate codes
    let referrer = Referrer::new(form_data.nickname.as_str(), "1234", "1234");
    let referrer_context =
        ReferrerContext {
            referrer: referrer.clone(),
            referrer_base64_link: Uri::percent_encode(base64_encode_referrer(referrer).as_str()).into_owned(),
            show_link: true,
        };

    dbg!(&referrer_context);

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
            Referrer::new("Sean", "1234", "4321"),
            Referrer::new("Jemma", "1234", "4321"),
        ],
        last_month: vec![Referrer::new("Grayson", "1234", "1234")],
    }
}

const BASE_64_SEPARATOR: char = '🔥';

fn base64_encode_referrer(referrer: Referrer) -> String {
    let concat = format!("{}{}{}{}{}", referrer.nickname, BASE_64_SEPARATOR, referrer.referrer_code, BASE_64_SEPARATOR, referrer.referrer_email_code);
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
                    if split.len() == 3 {
                        Ok(Referrer::new(split[0], split[1], split[2]))
                    } else {
                        Err(())
                    }
                }
            }
        }
    }
}