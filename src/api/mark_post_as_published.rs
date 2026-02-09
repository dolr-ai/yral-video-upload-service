use axum::{Json, extract::State};
use ic_agent::{Identity, identity::DelegatedIdentity};
use serde::{Deserialize, Serialize};
use yral_canisters_client::{
    ic::{USER_INFO_SERVICE_ID, USER_POST_SERVICE_ID},
    user_post_service::{self, PostStatus, Result2},
};

use crate::{
    app_state::AppState,
    utils::{
        notification_client::{self, NotificationType},
        types::{ApiResponse, DelegatedIdentityWire},
    },
};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MarkPostAsPublishedRequest {
    pub post_id: String,
    pub delegated_identity_wire: DelegatedIdentityWire,
}

pub async fn mark_post_as_published(
    State(app_state): State<AppState>,
    Json(payload): Json<MarkPostAsPublishedRequest>,
) -> ApiResponse<()> {
    let mark_post_as_published_res = mark_post_as_published_impl(
        &app_state.ic_admin_agent,
        &app_state.notification_client,
        &app_state.events_service,
        payload,
    )
    .await;

    ApiResponse::from(mark_post_as_published_res)
}

async fn mark_post_as_published_impl(
    ic_admin_agent: &ic_agent::Agent,
    notification_client: &notification_client::NotificationClient,
    event_service: &crate::utils::events_interface::EventService,
    payload: MarkPostAsPublishedRequest,
) -> Result<(), Box<dyn std::error::Error>> {
    let identity = DelegatedIdentity::try_from(payload.delegated_identity_wire)?;

    let user_post_service =
        user_post_service::UserPostService(USER_POST_SERVICE_ID, ic_admin_agent);

    let post_details_res = user_post_service
        .get_individual_post_details_by_id(payload.post_id.clone())
        .await?;

    let post_details = match post_details_res {
        Result2::Ok(post) => Ok(post),
        Result2::Err(user_post_service_error) => Err(format!(
            "Error from user post service while fetching post details for post id {}: {:?}",
            payload.post_id, user_post_service_error
        )),
    }?;

    if identity.sender()? != post_details.creator_principal {
        return Err(format!(
            "Unauthorized: the sender of the delegated identity is not the creator of the post. Sender: {:?}, Post Creator: {:?}",
            identity.sender()?,
            post_details.creator_principal
        )
        .into());
    }

    let mark_post_published_res = user_post_service
        .update_post_status(payload.post_id.clone(), PostStatus::Uploaded)
        .await;

    match mark_post_published_res {
        Ok(_) => {
            let _ = event_service
                .send_video_upload_successful_event(
                    post_details.video_uid,
                    post_details.hashtags.len(),
                    false,
                    true,
                    post_details.id.clone(),
                    post_details.creator_principal,
                    USER_INFO_SERVICE_ID,
                    String::new(),
                    None,
                )
                .await
                .inspect_err(|e| log::error!("error sending video upload successful event {e}"));
            notification_client
                .send_notification(
                    NotificationType::VideoPublished {
                        user_principal: post_details.creator_principal,
                        post_id: payload.post_id.clone(),
                    },
                    post_details.creator_principal,
                )
                .await;
            Ok(())
        }
        Err(e) => Err(e.into()),
    }
}
