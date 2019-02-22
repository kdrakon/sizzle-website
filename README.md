# The Rust Rocket sizzle-website

This is an implementation of the Sizzle website (https://thesizzle.com.au) that provides a referral function as described here: https://pastebin.com/raw/q4VgnPy0

This version of the website was migrated from a static one into a Rust-based web-framework called Rocket (https://rocket.rs). This was primarily done to support the server-side processing to accomplish the requirements of the referral page, called _refer-a-mate_.

## Breakdown of the Changes

Here's the breakdown of the changes:
1. Moved almost all the static webpages into the directory `static`. The existing `refer-a-mate.html` page was turned into a handlebars template since it would be a dynamically generated page for: 
    1. allowing people to generate a referral link,
    1. displaying this months leaderboard of referrers, as well as the winner of the previous month, and
    1. alternatively acting as the referral form for new subscribers
1. The Rust code (all in `src`) instantiates a web server that provides the following functionality:
    1. statically serving all the pages from `static`
    1. a `GET` and `POST` HTTP endpoint at `/mailchimp/subscribed?key=ABCD1234`
    1. a `GET` and `POST` HTTP endpoint at `/refer-a-mate` that serves the purpose already described above
    
### Mailchimp Integration
This `GET/POST` HTTP endpoint serves as a webhook/callback that can be configured in Mailchimp. Basically, whenever a subscription is completed, Mailchimp will make a `GET` request with an HTML form payload (which is usually reserved for `POST`'s, but whatever).

This payload can be customised to contain extra metadata (see link below). Assuming the Mailchimp account has been updated to support the extra metadata, the `refer-a-mate` subscribe form (described in the next section) will append that data. The app gets that info once the user has confirmed their subscription (i.e. when the webhook is triggered). The app saves the unique data to a database to keep track of subscription referrals. That is, for the leaderboard.

To protect this endpoint, the app and webhook both need to refer to a `key` that must be passed along with the request. Ideally, the app would be also updated to verify the origin of the caller. 

_See https://developer.mailchimp.com/documentation/mailchimp/guides/about-webhooks for more Mailchimp Webhook info._ 

### Refer-a-mate Implementation
This implementation is broken down into three sections.
#### The leaderboard (`GET /refer-a-mate`)
Quite simply, some database code is supplied to query the referrals to find two things:
1. an aggregation of all the referrals by referrers—i.e. grouped by their chosen nickname and their referrer code—are found for the current month. This is ordered by the number of subscriptions they referred that month, with a "tie-breaker" provided by a secondary ordering of who got the earliest referral.
1. a similar aggregation is executed to find the top referrer in the previous month. That person is the winner in that segment. By doing it this way, we don't need to effectively end competitions since the end of the month already acts as the "finish line".

Both of this data is computed and used in the `refer-a-mate` template for display.

#### Getting a refer-a-mate link  (`POST /refer-a-mate`)
The `refer-a-mate` template normally displays a new form that allows a person to enter their nickname and email. These details serve the purpose of uniquely identifying that person as a referrer.

When the user submits that HTML form, it calls the `POST` endpoint for `refer-a-mate` and deterministically generates a unique link that they can share. By using [HMAC](https://en.wikipedia.org/wiki/HMAC), I opted to not have any personal data—namely the referrers email address—stored anywhere in the database. By using a supplied secret key, the app encrypts the nickname and email address. The output is part of the referral code and can be used later when the user wants to claim ownership of the nickname. That is, in the event that they've won a month, they must provide their email address to verify they created the referral link that won.

#### The refer-a-mate Mailchimp form  (`GET /refer-a-mate?referrer=REFERRER_CODE`)
This form, which is an almost identical version from the Sizzle's homepage, was modified to embed the referral code and nickname of the referrer. This are retrievable via the referrer link, which is just a Base64 encoding of the data. This then covers the details as described in the Mailchimp integration above.
    
## Running the app
From cargo (the Rust build tool), the app can be started in development mode like so:
```$bash
cargo run -- -h HMAC_SECRET_KEY -m MAIL_CHIMP_KEY
```
This will use a Sqlite database located at `/tmp/sizzle_db`. By default, this will listen at http://localhost:8000.

A little more trivial work needs to be done to make the code production-ready.