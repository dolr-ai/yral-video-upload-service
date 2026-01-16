use axum::{Json, debug_handler, extract::State};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{app_state::AppState, utils::types::ApiResponse};

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
    let new_video_id = Uuid::new_v4();
    let result = app_state.storj_client.get_upload_url(
        &new_video_id.to_string(),
        &req.publisher_user_id,
        false,
    );

    ApiResponse {
        success: true,
        data: Some(GetUploadUrlResp { upload_url: result }),
        error_message: None,
        status_code: 200,
    }
}
