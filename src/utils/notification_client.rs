use std::fmt::Display;

use candid::Principal;
use serde::{Deserialize, Serialize};

const METADATA_SERVER_URL: &str = "https://metadata.yral.com";

#[derive(Clone, Debug)]
pub struct NotificationClient {
    api_key: String,
}

impl NotificationClient {
    pub fn new(api_key: String) -> Self {
        Self { api_key }
    }

    pub async fn send_notification(&self, data: NotificationType, user_principal: Principal) {
        let client = reqwest::Client::new();
        let url = format!(
            "{}/notifications/{}/send",
            METADATA_SERVER_URL,
            user_principal.to_text()
        );

        let title = data.to_string();
        let notification = Notification {
            notification: NotificationInfo {
                title: title,
                body: String::new(),
            },
            data,
        };

        let res = client
            .post(&url)
            .bearer_auth(&self.api_key)
            .json(&notification)
            .send()
            .await;

        match res {
            Ok(response) => {
                if response.status().is_success() {
                    log::info!(
                        "Notification sent successfully to user {}",
                        user_principal.to_text()
                    );
                } else if let Ok(body) = response.text().await {
                    let msg = format!(
                        "Failed to send notification to user {}: {}",
                        user_principal.to_text(),
                        body
                    );
                    log::error!("{}", msg);
                    sentry::capture_message(&msg, sentry::Level::Error);
                }
            }
            Err(req_err) => {
                let msg = format!(
                    "Error sending notification request to user {}: {}",
                    user_principal.to_text(),
                    req_err
                );
                log::error!("{}", msg);
                sentry::capture_message(&msg, sentry::Level::Error);
            }
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct NotificationInfo {
    pub title: String,
    pub body: String,
}

#[derive(Serialize, Deserialize)]
pub struct Notification {
    pub notification: NotificationInfo,
    pub data: NotificationType,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum NotificationType {
    VideoUploadedToDraft {
        user_principal: Principal,
        post_id: String,
    },
    VideoPublished {
        user_principal: Principal,
        post_id: String,
    },
}

impl Display for NotificationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NotificationType::VideoUploadedToDraft {
                user_principal: _user_principal,
                post_id: _post_id,
            } => {
                write!(
                    f,
                    "Your video was generated and added to Drafts in Profile section!"
                )
            }
            NotificationType::VideoPublished {
                user_principal: _user_principal,
                post_id: _post_id,
            } => {
                write!(f, "Your video has been published successfully")
            }
        }
    }
}
