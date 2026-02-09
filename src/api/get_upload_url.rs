use axum::{Json, extract::State};
use candid::Principal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use yral_canisters_client::{
    ic::USER_INFO_SERVICE_ID,
    user_info_service::{Result6 as UserCanisterProfileResult, UserInfoService},
};

use crate::{
    app_state::AppState,
    utils::{
        storj_interface::StorjInterface,
        types::{ApiResponse, AppError},
    },
};

#[derive(Serialize, Deserialize)]
pub struct GetUploadUrlReq {
    pub video_id: String,
    pub publisher_user_id: String,
}

#[derive(Serialize, Deserialize)]
pub struct GetUploadUrlResp {
    pub upload_url: String,
}

pub async fn get_upload_url(
    State(app_state): State<AppState>,
    Json(req): Json<GetUploadUrlReq>,
) -> ApiResponse<GetUploadUrlResp> {
    //TODO: check if the upload url created is for scheduled duration  yes it is scheduled
    //TODO: check if we need to first check if the user is present on our system.

    let get_upload_url_result =
        get_upload_url_impl(&app_state.ic_admin_agent, &app_state.storj_client, req).await;

    ApiResponse::from(get_upload_url_result)
}

async fn get_upload_url_impl(
    ic_admin_agent: &ic_agent::Agent,
    storj_client: &StorjInterface,
    req_data: GetUploadUrlReq,
) -> Result<GetUploadUrlResp, AppError> {
    let new_video_id = Uuid::new_v4();

    let user_principal = Principal::from_text(req_data.publisher_user_id.clone())?;

    let user_info_service = UserInfoService(USER_INFO_SERVICE_ID, ic_admin_agent);

    let profile_details_res = user_info_service
        .get_user_profile_details_v_6(user_principal)
        .await?;

    let _profile_details = match profile_details_res {
        UserCanisterProfileResult::Ok(profile_details) => profile_details,
        UserCanisterProfileResult::Err(e) => {
            log::error!("Failed to fetch user profile details: {}", e);
            return Err(AppError::UserProfileFetchError(e));
        }
    };

    let result = storj_client.get_upload_url(
        &new_video_id.to_string(),
        &req_data.publisher_user_id,
        false,
    );

    Ok(GetUploadUrlResp { upload_url: result })
}
