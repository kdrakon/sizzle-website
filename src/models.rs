pub struct HmacConfig {
    pub secret_key: String
}

pub struct MailchimpConfig {
    pub request_key: String
}

#[derive(FromForm, Debug)]
pub struct MailChimpSubscribeData {
    #[form(field = "type")]
    pub _type: String,
    #[form(field = "fired_at")]
    pub fired_at: String,
    #[form(field = "data%5Bid%5D")]
    pub id: String,
    #[form(field = "data%5Bemail%5D")]
    pub email: String,
    #[form(field = "data%5Bmerges%5D%5BREFERRER_CODE%5D")]
    pub referrer_code: String,
    #[form(field = "data%5Bmerges%5D%5BREFERRER_NICKNAME%5D")]
    pub referrer_nickname: String,
}

#[derive(Serialize)]
pub struct ReferAMateContext {
    pub top_referrers: TopReferrers,
    pub referrer_context: Option<ReferrerContext>,
}

#[derive(Serialize, Clone, Debug)]
pub struct Referrer {
    pub nickname: String,
    pub referrer_code: String,
}

impl Referrer {
    pub fn new(nickname: &str, referrer_code: &str) -> Referrer {
        let nickname = String::from(nickname);
        let referrer_code = String::from(referrer_code);
        Referrer { nickname, referrer_code }
    }
}

#[derive(Serialize, Debug)]
pub struct ReferrerContext {
    pub referrer: Referrer,
    pub referrer_base64_link: String,
    pub show_link: bool,
}

#[derive(Serialize)]
pub struct TopReferrers {
    pub this_month: Vec<Referrer>,
    pub last_month: Vec<Referrer>,
}

impl TopReferrers {
    pub fn empty() -> TopReferrers {
        TopReferrers { this_month: vec![], last_month: vec![] }
    }
}

#[derive(FromForm)]
pub struct ReferAMateFormData {
    pub email: String,
    pub nickname: String,
}
