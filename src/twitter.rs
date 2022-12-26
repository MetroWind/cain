use std::cell::Cell;

use ureq;
use serde_json;
use url::Url;
use log::warn;

use crate::error::Error;
use crate::runtime_config;
use crate::analyser;
use crate::analyser::TempItem;

static TWITTER_TOKEN_KEY: &str = "twitter_token";
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

struct TokenManager
{
    token: Cell<String>,
}

impl TokenManager
{
    fn new() -> Result<Self, Error>
    {
        if let Some(token) = runtime_config::get(TWITTER_TOKEN_KEY)?
        {
            Ok(Self { token: Cell::new(token) })
        }
        else
        {
            let token = newGuestToken()?;
            if let Err(e) = runtime_config::set(TWITTER_TOKEN_KEY, &token)
            {
                warn!("Failed to set runtime config: {}", e);
            }
            Ok(Self { token: Cell::new(token) })
        }
    }

    fn get(&self) -> String
    {
        let v = self.token.take();
        self.token.set(v.clone());
        v
    }

    fn refresh(&self) -> Result<String, Error>
    {
        self.token.set(newGuestToken()?);
        Ok(self.get())
    }
}

pub struct Client
{
    token: TokenManager,
}

impl Client
{
    pub fn new() -> Result<Self, Error>
    {
        Ok(Self { token: TokenManager::new()? })
    }

    fn getTweet(&self, id: &str) -> Result<serde_json::Value, Error>
    {
        let req = ureq::get(&format!(
            "https://api.twitter.com/1.1/statuses/show.json?id={}", id))
            .set(AUTH_HEADER_KEY, GUEST_AUTH)
            .set(GUEST_TOKEN_HEADER_KEY, &self.token.get());
        let res = req.clone().call()
            .map_err(|_| rterr!("Failed to get tweet"))?;
        let res = if res.status() == 401
        {
            req.set(GUEST_TOKEN_HEADER_KEY, &self.token.refresh()?).call()
                .map_err(|_| rterr!("Failed to get tweet"))?
        }
        else
        {
            res
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
        let client = Client::new()?;
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
