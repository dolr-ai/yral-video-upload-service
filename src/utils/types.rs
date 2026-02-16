use axum::body::Body;
use axum::response::{IntoResponse, Response};
use candid::Principal;
use ic_agent::identity::SignedDelegation;
use ic_agent::identity::{DelegatedIdentity, Secp256k1Identity};
use k256::elliptic_curve::JwkEcKey;
use reqwest::header::CONTENT_TYPE;
use serde::{Deserialize, Serialize};
use std::error::Error;
use thiserror::Error;
use utoipa::openapi::schema::{self};
use utoipa::openapi::{ArrayBuilder, Object, ObjectBuilder};
use utoipa::{PartialSchema, ToSchema};
use yral_canisters_client::user_post_service::{PostDetailsFromFrontendV1, PostStatusFromFrontend};

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Invalid principal: {0}")]
    InvalidPrincipal(String),

    #[error("Failed to fetch user profile: {0}")]
    UserProfileFetchError(String),

    #[error("User not found")]
    UserNotFound,

    #[error("Storage error: {0}")]
    StorageError(String),

    #[error("Internal server error: {0}")]
    InternalError(String),

    #[error("Agent error: {0}")]
    AgentError(String),

    #[error("Invalid delegated identity: {0}")]
    InvalidDelegatedIdentity(String),

    #[error("Post not found: {0}")]
    PostNotFound(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Canister error: {0}")]
    CanisterError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),
}

impl From<ic_agent::agent::AgentError> for AppError {
    fn from(error: ic_agent::agent::AgentError) -> Self {
        AppError::AgentError(error.to_string())
    }
}

impl From<candid::error::Error> for AppError {
    fn from(error: candid::error::Error) -> Self {
        AppError::InvalidPrincipal(error.to_string())
    }
}

impl From<candid::types::principal::PrincipalError> for AppError {
    fn from(error: candid::types::principal::PrincipalError) -> Self {
        AppError::InvalidPrincipal(error.to_string())
    }
}

impl From<Box<dyn Error>> for AppError {
    fn from(error: Box<dyn Error>) -> Self {
        AppError::InternalError(error.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(error: serde_json::Error) -> Self {
        AppError::SerializationError(error.to_string())
    }
}

impl AppError {
    pub fn status_code(&self) -> u16 {
        match self {
            AppError::InvalidPrincipal(_) => 400,
            AppError::UserProfileFetchError(_) => 400,
            AppError::UserNotFound => 404,
            AppError::StorageError(_) => 503,
            AppError::InternalError(_) => 500,
            AppError::AgentError(_) => 502,
            AppError::InvalidDelegatedIdentity(_) => 400,
            AppError::PostNotFound(_) => 404,
            AppError::Unauthorized(_) => 403,
            AppError::CanisterError(_) => 502,
            AppError::SerializationError(_) => 500,
        }
    }

    pub fn to_api_response<T>(&self) -> ApiResponse<T> {
        ApiResponse {
            success: false,
            data: None,
            error_message: Some(self.to_string()),
            status_code: self.status_code(),
        }
    }
}

impl<T: Serialize> From<Result<T, AppError>> for ApiResponse<T> {
    fn from(result: Result<T, AppError>) -> Self {
        match result {
            Ok(data) => ApiResponse {
                success: true,
                data: Some(data),
                error_message: None,
                status_code: 200,
            },
            Err(e) => e.to_api_response(),
        }
    }
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct EmptyResp {}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error_message: Option<String>,
    #[serde(skip_serializing, default)]
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

#[derive(Serialize, Deserialize, Clone, Debug)]
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

impl ToSchema for DelegatedIdentityWire {
    fn name() -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Borrowed("DelegatedIdentityWire")
    }
}

impl PartialSchema for DelegatedIdentityWire {
    fn schema() -> utoipa::openapi::RefOr<utoipa::openapi::schema::Schema> {
        utoipa::openapi::ObjectBuilder::new()
            .property(
                "from_key",
                ArrayBuilder::new()
                    .items(Object::with_type(schema::Type::Number))
                    .description("Raw bytes of the delegated identity's public key. This is the key that is being delegated from.".into())
            )
            .property(
                "to_secret",
                utoipa::openapi::Schema::Object(
                    ObjectBuilder::new()
                        .property(
                            "kty",
                            ObjectBuilder::new().schema_type(utoipa::openapi::schema::Type::String)
                        )
                        .property(
                            "crv",
                            ObjectBuilder::new().schema_type(utoipa::openapi::schema::Type::String)
                        )
                        .property(
                            "d",
                            ObjectBuilder::new().schema_type(utoipa::openapi::schema::Type::String)
                        )
                        .description("JWK(JSON Web Key) encoded Secp256k1 secret key of the identity allowed to sign on behalf of `from_key`.".into())
                        .into()
                )
            )
            .property(
                "delegation_chain",
                ObjectBuilder::new()
                    .schema_type(utoipa::openapi::schema::Type::Array)
                    .description("Proof of delegation connecting `from_key` to `to_secret`.".into())
            )
            .into()
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

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;
    use ic_agent::{
        Identity,
        identity::{DelegatedIdentity, Delegation, SignedDelegation},
    };
    use k256::{SecretKey, elliptic_curve::rand_core::OsRng, pkcs8::EncodePublicKey};

    fn create_delegated_identity_wire(
        from_key: impl Identity,
        to_key: SecretKey,
    ) -> DelegatedIdentityWire {
        let delegation = Delegation {
            pubkey: to_key
                .public_key()
                .to_public_key_der()
                .unwrap()
                .as_bytes()
                .to_vec(),
            expiration: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
                + 3600, // valid for 1 hour
            targets: None,
        };

        let delegation_signature = from_key
            .sign_delegation(&delegation)
            .expect("Failed to sign delegation");

        let signed_delegation = SignedDelegation {
            delegation,
            signature: delegation_signature.signature.unwrap(),
        };

        let mut delegation_chain = delegation_signature.delegations.unwrap_or_default();

        delegation_chain.push(signed_delegation);

        DelegatedIdentityWire {
            from_key: from_key.public_key().unwrap(),
            to_secret: to_key.to_jwk(),
            delegation_chain,
        }
    }

    #[test]
    fn test_valid_delegated_identity_wire() {
        let from_main_secret_key = SecretKey::random(&mut OsRng);
        let from_secret_key = SecretKey::random(&mut OsRng);
        let to_secret_key = SecretKey::random(&mut OsRng);

        let main_key = Secp256k1Identity::from_private_key(from_main_secret_key);

        let from_delegated_identity_wire =
            create_delegated_identity_wire(main_key, from_secret_key);

        let delegated_identity = DelegatedIdentity::try_from(from_delegated_identity_wire.clone())
            .expect("Failed to create delegated identity from wire format");

        let to_key_delegated_identity_wire =
            create_delegated_identity_wire(delegated_identity, to_secret_key);

        let _to_key_delegated_identity =
            DelegatedIdentity::try_from(to_key_delegated_identity_wire.clone())
                .expect("Failed to create delegated identity from wire format");
    }
}
