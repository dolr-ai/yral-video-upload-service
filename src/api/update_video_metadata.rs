use std::{collections::HashMap, error::Error};

use axum::{Json, extract::State};
use ic_agent::{Identity, identity::DelegatedIdentity};
use serde::Deserialize;
use yral_canisters_client::{
    ic::{self, USER_POST_SERVICE_ID},
    user_post_service::{
        PostDetailsFromFrontend, PostDetailsFromFrontendV1, Result_, UserPostService,
    },
};

use crate::{
    app_state::{self, AppState},
    utils::{
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
    match update_metadata_impl_v2(&app_state.ic_admin_agent, &app_state.storj_client, req).await {
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

async fn update_metadata_impl_v2(
    ic_admin_agent: &ic_agent::Agent,
    storj_interface: &StorjInterface,
    mut req_data: UpdateMetadataRequest,
) -> Result<(), Box<dyn Error>> {
    let delegated_identity = DelegatedIdentity::try_from(req_data.delegated_identity_wire.clone())?;

    //TODO: we not using delegated identity for storj upload or canister upload we could get away with a signature that is signed by this Delegated Identity.

    let publisher_user_id = delegated_identity.sender()?.to_text();

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

    upload_video_canister(ic_admin_agent, req_data.post_details.clone()).await?;

    //TODO: send notification to the client

    Ok(())
}

async fn upload_video_canister(
    ic_admin_agent: &ic_agent::Agent,
    post_details: PostDetailsFromFrontendV1,
) -> Result<(), Box<dyn Error>> {
    let user_post_service_canister = UserPostService(USER_POST_SERVICE_ID, ic_admin_agent);

    let upload_to_canister_res = user_post_service_canister
        .add_post_v_1(post_details)
        .await?;

    match upload_to_canister_res {
        Result_::Ok => Ok(()),
        Result_::Err(user_post_service_error) => {
            Err(format!("Canister error: {:?}", user_post_service_error).into())
        }
    }
}
