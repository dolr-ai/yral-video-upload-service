use std::{collections::HashMap, error::Error};

use axum::{Json, extract::State};
use ic_agent::{Identity, identity::DelegatedIdentity};
use serde::Deserialize;
use yral_canisters_client::{
    ic::{USER_INFO_SERVICE_ID, USER_POST_SERVICE_ID},
    user_post_service::{
        PostDetailsFromFrontendV1, PostStatus, PostStatusFromFrontend, Result_, UserPostService,
    },
};
use yral_types::post;

use crate::{
    app_state::AppState,
    utils::{
        events_interface::EventService,
        notification_client::{self, NotificationClient, NotificationType},
        storj_interface::StorjInterface,
        types::{ApiResponse, DelegatedIdentityWire, RequestPostDetails},
    },
};

pub static POST_DETAILS_KEY: &str = "post_details";

pub async fn update_video_metadata(
    State(app_state): State<AppState>,
    Json(req): Json<UpdateMetadataRequest>,
) -> ApiResponse<()> {
    //TODO: send event
    //TODO: send notification
    match update_metadata_impl(
        &app_state.ic_admin_agent,
        &app_state.storj_client,
        &app_state.events_service,
        &app_state.notification_client,
        req,
    )
    .await
    {
        Ok(_) => ApiResponse {
            success: true,
            data: Some(()),
            error_message: None,
            status_code: 200,
        },
        Err(e) => ApiResponse {
            success: false,
            data: None,
            error_message: Some(e.to_string()),
            status_code: 500,
        },
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateMetadataRequest {
    pub video_uid: String,
    pub delegated_identity_wire: DelegatedIdentityWire,
    pub meta: HashMap<String, String>,
    pub post_details: PostDetailsFromFrontendV1,
}

async fn update_metadata_impl(
    ic_admin_agent: &ic_agent::Agent,
    storj_interface: &StorjInterface,
    events_service: &EventService,
    notification_client: &NotificationClient,
    mut req_data: UpdateMetadataRequest,
) -> Result<(), Box<dyn Error>> {
    let delegated_identity = DelegatedIdentity::try_from(req_data.delegated_identity_wire.clone())?;

    //TODO: we not using delegated identity for storj upload or canister upload we could get away with a signature that is signed by this Delegated Identity.

    let publisher_user_id = delegated_identity.sender()?.to_text();

    if !publisher_user_id.eq(&req_data.post_details.creator_principal.to_text()) {
        return Err("Publisher user id does not match creator principal in post details".into());
    }

    req_data.meta.insert(
        POST_DETAILS_KEY.to_string(),
        serde_json::to_string(&Into::<RequestPostDetails>::into(
            req_data.post_details.clone(),
        ))?,
    );

    // Finalize Storj upload with metadata (without delegated-identity)
    storj_interface
        .finalize_upload(
            &req_data.video_uid,
            &publisher_user_id,
            false,
            req_data.meta.clone(),
        )
        .await?;

    upload_video_canister(
        ic_admin_agent,
        events_service,
        notification_client,
        req_data.post_details.clone(),
    )
    .await?;

    Ok(())
}

async fn upload_video_canister(
    ic_admin_agent: &ic_agent::Agent,
    events_service: &EventService,
    notification_client: &NotificationClient,
    post_details: PostDetailsFromFrontendV1,
) -> Result<(), Box<dyn Error>> {
    let user_post_service_canister = UserPostService(USER_POST_SERVICE_ID, ic_admin_agent);

    let post_is_published = matches!(post_details.status, PostStatusFromFrontend::Published);

    let upload_to_canister_res = user_post_service_canister
        .add_post_v_1(post_details.clone())
        .await?;

    match upload_to_canister_res {
        Result_::Ok => {
            if post_is_published {
                let _ = events_service
                    .send_video_upload_successful_event(
                        post_details.video_uid,
                        post_details.hashtags.len(),
                        false,
                        true,
                        post_details.id.clone(),
                        post_details.creator_principal,
                        USER_INFO_SERVICE_ID.into(),
                        String::new(),
                        None,
                    )
                    .await
                    .inspect_err(|e| {
                        log::error!("Failed to send video_upload_successful event: {}", e);
                    });
            }

            let notification_payload = if post_is_published {
                NotificationType::VideoPublished {
                    user_principal: post_details.creator_principal,
                    post_id: post_details.id.clone(),
                }
            } else {
                NotificationType::VideoUploadedToDraft {
                    user_principal: post_details.creator_principal,
                    post_id: post_details.id.clone(),
                }
            };

            notification_client
                .send_notification(notification_payload, post_details.creator_principal)
                .await;

            Ok(())
        }
        Result_::Err(user_post_service_error) => {
            let error = format!("Canister error: {:?}", user_post_service_error);

            let _ = events_service
                .send_video_event_unsuccessful(
                    error.clone(),
                    post_details.hashtags.len(),
                    false,
                    true,
                    post_details.creator_principal,
                    String::new(),
                    USER_INFO_SERVICE_ID.into(),
                )
                .await
                .inspect_err(|e| {
                    log::error!("Failed to send video_event_unsuccessful event: {}", e)
                });

            Err(error.into())
        }
    }
}
