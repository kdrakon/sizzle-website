#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;
extern crate rocket_contrib;
#[macro_use]
extern crate serde;

use rocket::http::Status;
use rocket::request::LenientForm;
use rocket_contrib::serve::*;
use rocket_contrib::templates::Template;
use serde::ser::Serialize;

fn main() {
    rocket
    ::ignite()
        .mount("/", routes![mailchimp_subscribed_get_webhook, mailchimp_subscribed_post_webhook, refer_a_mate])
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
    #[form(field = "data%5Bmerges%5D%5BREFERRER_EMAIL%5D")]
    referrer_email: String,
    #[form(field = "data%5Bmerges%5D%5BREFERRER_NICKNAME%5D")]
    referrer_nickname: String,
}

#[get("/mailchimp/subscribed", data = "<mailchimp_subscribed_data>")]
fn mailchimp_subscribed_get_webhook(mailchimp_subscribed_data: LenientForm<MailChimpSubscribeData>) -> Result<Status, Status> {
    mailchimp_subscribed_post_webhook(mailchimp_subscribed_data)
}

#[post("/mailchimp/subscribed", data = "<mailchimp_subscribed_data>")]
fn mailchimp_subscribed_post_webhook(mailchimp_subscribed_data: LenientForm<MailChimpSubscribeData>) -> Result<Status, Status> {
    Ok(Status::Accepted)
}

#[derive(Serialize)]
struct ReferAMateContext {
    ok: bool
}

#[get("/refer-a-mate")]
fn refer_a_mate() -> Template {
    let context: ReferAMateContext = ReferAMateContext { ok: true };
    Template::render("refer-a-mate", context)
}