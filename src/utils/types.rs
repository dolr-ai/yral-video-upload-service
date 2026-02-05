use std::error::Error;

use axum::body::Body;
use axum::response::{IntoResponse, Response};
use candid::Principal;
use ic_agent::identity::{DelegatedIdentity, Secp256k1Identity, SignedDelegation};
use k256::elliptic_curve::JwkEcKey;
use reqwest::header::CONTENT_TYPE;
use serde::{Deserialize, Serialize};
use yral_canisters_client::user_post_service::{PostDetailsFromFrontendV1, PostStatusFromFrontend};

#[derive(Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error_message: Option<String>,
    #[serde(skip_serializing)]
    pub status_code: u16,
}

impl<T: Serialize> IntoResponse for ApiResponse<T> {
    fn into_response(self) -> Response {
        //todo we need to think about response status

        let response = Response::builder()
            .header(CONTENT_TYPE, "application/json")
            .status(self.status_code)
            .body(Body::from(serde_json::to_string(&self).unwrap()))
            .unwrap();

        response
    }
}

impl<T: Serialize> From<Result<T, Box<dyn Error>>> for ApiResponse<T> {
    fn from(result: Result<T, Box<dyn Error>>) -> Self {
        match result {
            Ok(data) => ApiResponse {
                success: true,
                data: Some(data),
                error_message: None,
                status_code: 200,
            },
            Err(e) => ApiResponse {
                success: false,
                data: None,
                error_message: Some(e.to_string()),
                status_code: 400,
            },
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct DelegatedIdentityWire {
    /// raw bytes of delegated identity's public key
    pub from_key: Vec<u8>,
    /// JWK(JSON Web Key) encoded Secp256k1 secret key
    /// identity allowed to sign on behalf of `from_key`
    pub to_secret: JwkEcKey,
    /// Proof of delegation
    /// connecting from_key to `to_secret`
    pub delegation_chain: Vec<SignedDelegation>,
}

impl std::fmt::Debug for DelegatedIdentityWire {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DelegatedIdentityWire").finish()
    }
}

impl TryFrom<DelegatedIdentityWire> for DelegatedIdentity {
    type Error = Box<dyn Error>;

    fn try_from(value: DelegatedIdentityWire) -> Result<DelegatedIdentity, Box<dyn Error>> {
        let to_secret = k256::SecretKey::from_jwk(&value.to_secret)?;
        let to_identity = Secp256k1Identity::from_private_key(to_secret);
        Self::new(
            value.from_key,
            Box::new(to_identity),
            value.delegation_chain,
        )
        .map_err(|e| Box::new(e) as Box<dyn Error>)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestPostDetails {
    pub video_uid: String,
    pub description: String,
    pub hashtags: Vec<String>,
    pub creator_principal: Principal,
    pub id: String,
}

impl From<PostDetailsFromFrontendV1> for RequestPostDetails {
    fn from(value: PostDetailsFromFrontendV1) -> Self {
        Self {
            video_uid: value.video_uid,
            description: value.description,
            hashtags: value.hashtags,
            id: value.id,
            creator_principal: value.creator_principal,
        }
    }
}

impl From<RequestPostDetails> for PostDetailsFromFrontendV1 {
    fn from(value: RequestPostDetails) -> Self {
        Self {
            video_uid: value.video_uid,
            description: value.description,
            hashtags: value.hashtags,
            id: value.id,
            creator_principal: value.creator_principal,
            status: PostStatusFromFrontend::Draft,
        }
    }
}
