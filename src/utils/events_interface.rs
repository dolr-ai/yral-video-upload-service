use std::error::Error;

use axum::http::{HeaderMap, HeaderValue};
use candid::Principal;
use reqwest::{Client, ClientBuilder, Url, header};
use serde_json::json;

#[derive(Clone)]
pub struct EventService {
    base_url: Url,
    reqwest_client: Client,
}

impl EventService {
    pub fn with_auth_token(auth_token: String) -> Self {
        let base_url = "https://offchain.yral.com/";
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_str(format!("Bearer {}", &auth_token).as_ref()).unwrap(),
        );
        Self {
            reqwest_client: ClientBuilder::new()
                .default_headers(headers)
                .build()
                .expect("Invalid event service client config"),
            base_url: Url::parse(base_url).unwrap(),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn send_video_upload_successful_event(
        &self,
        video_uid: String,
        hashtags_len: usize,
        is_nsfw: bool,
        enable_hot_or_not: bool,
        post_id: String,
        user_principal: Principal,
        canister_id: Principal,
        user_name: String,
        country: Option<String>,
    ) -> Result<(), Box<dyn Error>> {
        let params = json!({
            "user_id": user_principal,
            "publisher_user_id": user_principal,
            "display_name": user_name,
            "canister_id": canister_id,
            "creator_category": "NA",
            "hashtag_count": hashtags_len,
            "is_nsfw": is_nsfw,
            "is_hot_or_not": enable_hot_or_not,
            "is_filter_used": false,
            "video_id": video_uid,
            "post_id": post_id,
            "country": country,
        })
        .to_string();

        let path = "api/v2/events";

        let response = self
            .reqwest_client
            .post(self.base_url.join(path).unwrap())
            .json(&json!({
                "event": "video_upload_successful".to_owned(),
                "params": params
            }))
            .send()
            .await?;

        if response.status().is_success() {
            Ok(())
        } else {
            let status = response.status();
            let error = response.text().await?;
            Err(
                format!("error sending video_upload_successful event. Error {status} {error}",)
                    .into(),
            )
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn send_video_event_unsuccessful(
        &self,
        error: String,
        hashtags_len: usize,
        is_nsfw: bool,
        enable_hot_or_not: bool,
        user_principal: Principal,
        user_name: String,
        user_canister: Principal,
    ) -> Result<(), Box<dyn Error>> {
        let params = json!({
            "user_id": user_principal,
            "display_name": user_name,
            "canister_id": user_canister,
            "creator_category": "NA",
            "hashtag_count": hashtags_len,
            "is_NSFW": is_nsfw,
            "is_hotorNot": enable_hot_or_not,
            "fail_reason": error,
        })
        .to_string();

        let path = "api/v2/events";

        let response = self
            .reqwest_client
            .post(self.base_url.join(path).unwrap())
            .json(&json!({
                "event": "video_upload_unsuccessful".to_owned(),
                "params": params
            }))
            .send()
            .await?;

        if response.status().is_success() {
            Ok(())
        } else {
            let status = response.status();
            let error = response.text().await?;
            Err(
                format!("error sending video_upload_successful event. Error {status} {error}",)
                    .into(),
            )
        }
    }
}
