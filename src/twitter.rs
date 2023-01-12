use std::cell::Cell;
use std::io::Read;
use std::str;

use ureq;
use serde_json;
use url::Url;
use log::{info, warn, debug};
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use hmac::Mac;

use crate::error::Error;
use crate::runtime_config;
use crate::analyser;
use crate::analyser::TempItem;

static GUEST_TOKEN_KEY: &str = "twitter_guest_token";
static GUEST_AUTH: &str = "Bearer AAAAAAAAAAAAAAAAAAAAANRILgAAAAAAnNwIzUejRCOuH5E6I8xnZz4puTs%3D1Zv7ttfk8LF81IUq16cHjhLTvJu4FA33AGWWjCpTnA";
static AUTH_HEADER_KEY: &str = "Authorization";
static GUEST_TOKEN_HEADER_KEY: &str = "X-guest-token";

fn newGuestToken() -> Result<String, Error>
{
    let res =
        ureq::post("https://api.twitter.com/1.1/guest/activate.json")
        .set(AUTH_HEADER_KEY, GUEST_AUTH)
        .call().map_err(|_| rterr!("Failed to get guest token"))?
        .into_string().map_err(
            |_| rterr!("Failed to encode guest token response"))?;
    let data: serde_json::Value = res.parse()
        .map_err(|_| rterr!("Failed to serialize guest token response"))?;
    Ok(data["guest_token"].as_str()
       .ok_or_else(|| rterr!("Invalid guest token response"))?.to_owned())
}

fn percentEncode(s: &str) -> String
{
    utf8_percent_encode(s, NON_ALPHANUMERIC).to_string()
}

pub trait TokenManager
{
    /// Request a new set of tokens
    fn authenticate(&self) -> Result<(), Error>;
    fn decorated(&self, req: ureq::Request) -> Result<ureq::Request, Error>;
    /// Whether to reauthenticate if the existing token is invalid.
    fn reauthenticate(&self) -> bool {false}
}

pub struct GuestToken
{
    token: Cell<String>,
}

impl GuestToken
{
    pub fn new() -> Result<Self, Error>
    {
        if let Some(token) = runtime_config::get(GUEST_TOKEN_KEY)?
        {
            Ok(Self { token: Cell::new(token) })
        }
        else
        {
            let t = Self { token: Cell::default() };
            t.authenticate()?;
            Ok(t)
        }
    }
}

impl TokenManager for GuestToken
{
    fn authenticate(&self) -> Result<(), Error>
    {
        let token = newGuestToken()?;
        if let Err(e) = runtime_config::set(GUEST_TOKEN_KEY, &token)
        {
            warn!("Failed to set runtime config: {}", e);
        }
        self.token.set(token);
        Ok(())
    }

    fn decorated(&self, req: ureq::Request) -> Result<ureq::Request, Error>
    {
        let t = self.token.take();
        let r = req.set(AUTH_HEADER_KEY, GUEST_AUTH)
            .set(GUEST_TOKEN_HEADER_KEY, &t);
        self.token.set(t);
        Ok(r)
    }

    fn reauthenticate(&self) -> bool {true}
}

pub struct StaticToken
{
    pub consumer_key: String,
    pub consumer_secret: String,
    pub access_token: String,
    pub access_token_secret: String,
}

impl StaticToken
{
    fn nonce(time_stamp: i64) -> String
    {
        let i: u32 = rand::random();
        format!("{:08x}{:x}", i, time_stamp)
    }

    fn getAuthValue(&self, req: &ureq::Request) -> Result<String, Error>
    {
        // Build auth signature. See
        // https://developer.twitter.com/en/docs/authentication/oauth-1-0a/creating-a-signature
        let time_stamp = time::OffsetDateTime::now_utc().unix_timestamp();
        let time_stamp_str = time_stamp.to_string();
        let nonce = Self::nonce(time_stamp);
        debug!("Nonce is {}.", nonce);
        let url = req.request_url().map_err(|_| rterr!("Invalid URL"))?;
        let mut kvs = url.query_pairs();
        kvs.push(("oauth_consumer_key", &self.consumer_key));
        kvs.push(("oauth_nonce", &nonce));
        kvs.push(("oauth_signature_method", "HMAC-SHA1"));
        kvs.push(("oauth_timestamp", &time_stamp_str));
        kvs.push(("oauth_token", &self.access_token));
        kvs.push(("oauth_version", "1.0"));

        let mut encoded: Vec<String> = kvs.iter().map(
            |(k, v)| format!("{}={}", percentEncode(k), percentEncode(v)))
            .collect();
        encoded.sort();
        let kvs = encoded.join("&");

        let base: String = format!(
            "{}&{}&{}", req.method(), percentEncode(url.as_url().as_str()),
            percentEncode(&kvs));
        let signing_key = format!("{}&{}", percentEncode(&self.consumer_secret),
                                  percentEncode(&self.access_token_secret));
        type HmacSha1 = hmac::Hmac::<sha1::Sha1>;
        let mut hmac = HmacSha1::new_from_slice(signing_key.as_bytes()).map_err(
            |_| rterr!("Failed to create hmac core"))?;
        hmac.update(base.as_bytes());
        let sig = base64::encode(&hmac.finalize().into_bytes());
        debug!("Oauth signature is {}.", sig);
        // Build auth header
        let auth_value = format!(r#"OAuth oauth_consumer_key="{}", oauth_nonce="{}", oauth_signature="{}", oauth_signature_method="HMAC-SHA1", oauth_timestamp="{}", oauth_token="{}", oauth_version="1.0""#,
                                 percentEncode(&self.consumer_key),
                                 percentEncode(&nonce),
                                 percentEncode(&sig),
                                 time_stamp_str,
                                 percentEncode(&self.access_token));
        debug!("Auth header is {}", auth_value);
        Ok(auth_value)
    }
}

impl TokenManager for StaticToken
{
    fn authenticate(&self) -> Result<(), Error>
    {
        Ok(())
    }

    fn decorated(&self, req: ureq::Request) -> Result<ureq::Request, Error>
    {
        let auth_value = self.getAuthValue(&req)?;
        Ok(req.set(AUTH_HEADER_KEY, &auth_value))
    }
}

pub struct Client
{
    token: Box<dyn TokenManager>,
}

impl Client
{
    pub fn new<T>(t: T) -> Result<Self, Error>
        where T: TokenManager + 'static
    {
        Ok(Self { token: Box::new(t) })
    }

    fn getTweet(&self, id: &str) -> Result<serde_json::Value, Error>
    {
        // Twitter error response does not have Content-Length, and by
        // default ureq will wait for the server to close socket when
        // reading. An agent can have a read timeout.
        let agent = ureq::builder()
            .timeout_read(std::time::Duration::from_secs(10))
            .build();

        let req = self.token.decorated(agent.get(&format!(
            "https://api.twitter.com/1.1/statuses/show.json?id={}", id)))?;
        debug!("Sending request...");
        let res = match req.clone().call()
        {
            Ok(res) => res,
            Err(ureq::Error::Status(401, res)) |
            Err(ureq::Error::Status(403, res)) =>
            {
                let mut bytes: Vec<u8> = Vec::with_capacity(1024);
                // This could timeout.
                debug!("Reading response...");
                let _ = res.into_reader().take(10_000_000)
                    .read_to_end(&mut bytes);
                debug!("Done.");
                let payload = str::from_utf8(&bytes).unwrap();

                if self.token.reauthenticate()
                {
                    self.token.authenticate()?;
                    self.token.decorated(req)?.call()
                        .map_err(|_| rterr!(
                            "Failed to get tweet with refreshed token"))?
                }
                else
                {
                    return Err(rterr!("Invalid token: {}", payload));
                }
            },
            Err(ureq::Error::Status(code, res)) =>
            {
                let mut bytes: Vec<u8> = Vec::with_capacity(1024);
                // This could timeout.
                let _ = res.into_reader().take(10_000_000)
                    .read_to_end(&mut bytes);
                let payload = str::from_utf8(&bytes).unwrap();
                let err = rterr!(
                    "Failed to get tweet with code {} and error: {}",
                    code, payload);
                return Err(err);
            },
            Err(_) =>
            {
                return Err(rterr!("Failed to get tweet"));
            },
        };

        let body = res.into_string().map_err(
                |_| rterr!("Failed to encode tweet response"))?;
        let data: serde_json::Value = body.parse()
            .map_err(|_| rterr!("Failed to serialize tweet response"))?;

        Ok(data)
    }
}

fn getTweetMedia(media_data: &serde_json::Value) ->
    Result<Option<TempItem>, Error>
{
    match media_data["type"].as_str()
        .ok_or_else(|| rterr!("Failed to get tweet media type"))?
    {
        "video" =>
        {
            let variants = media_data["video_info"]["variants"].as_array()
                .ok_or_else(|| rterr!("Failed to get tweet video variants"))?;
            let info = variants.iter()
                .max_by_key(|v| v["bitrate"].as_i64().or(Some(0)).unwrap())
                .ok_or_else(|| rterr!("Empty tweet video variants"))?;
            let url = info["url"].as_str().ok_or_else(
                || rterr!("Tweet video variant does not have URL"))?;
            Ok(Some(TempItem::Url(url.to_owned())))
        },
        "photo" =>
        {
            let url = media_data["media_url"].as_str().ok_or_else(
                || rterr!("Tweet photo does not have URL"))?;
            Ok(Some(TempItem::Url(url.to_owned())))
        },
        _ => Ok(None),
    }
}

impl analyser::ResourceAnalyser for Client
{
    fn analyse(&self, url: &str) -> Result<Vec<TempItem>, Error>
    {
        let mut resources = Vec::new();
        let u = Url::parse(url).map_err(|_| rterr!("Invalid URL: {}", url))?;
        let id = u.path_segments()
            .ok_or_else(|| rterr!("Invalid Tweet URL: {}", url))?
            .last().ok_or_else(|| rterr!("Invalid Tweet URL: {}", url))?;
        let data = self.getTweet(id)?;
        resources.push(
            TempItem::Text(data["text"].as_str()
                           .ok_or_else(|| rterr!("Failed to get tweet text"))?
                           .to_owned()));
        if let Some(medias) = data["extended_entities"]["media"].as_array()
        {
            for media in medias
            {
                if let Some(stuff) = getTweetMedia(media)?
                {
                    if let TempItem::Url(u) = &stuff
                    {
                        info!("Found Twitter media at {}.", u);
                    }
                    resources.push(stuff);
                }
            }
        }
        Ok(resources)
    }
}

#[cfg(test)]
mod tests
{
    use crate::analyser::ResourceAnalyser;

    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    #[test]
    fn analyse() -> Result<(), Error>
    {
        let client = Client::new(GuestToken::new()?)?;
        let items = client.analyse("https://twitter.com/MetroWind/status/1595694065353248768")?;
        assert_eq!(items.len(), 2);
        assert_eq!(items[0], TempItem::Text("刚刚剁手了这些…… https://t.co/WRoKBpQXyb".to_owned()));
        match items[1]
        {
            TempItem::Url(_) => assert!(true),
            _ => assert!(false),
        }
        Ok(())
    }
}
