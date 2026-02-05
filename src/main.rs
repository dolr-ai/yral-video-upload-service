use std::{env, sync::Arc};

use axum::{
    Router,
    routing::{get, post},
};

use crate::{
    api::get_upload_url::get_upload_url,
    app_state::AppState,
    utils::{
        events_interface::EventService, notification_client::NotificationClient,
        storj_interface::StorjInterface,
    },
};

pub mod api;
pub mod app_state;
pub mod utils;

#[tokio::main]
async fn main() {
    env_logger::init();
    let ic_admin_identity = {
        #[cfg(feature = "ic-admin")]
        {
            use ic_agent::identity::Secp256k1Identity;
            use k256::Secp256k1;

            let private_key = k256::SecretKey::from_be_bytes(
                &hex::decode(std::env::var("IC_ADMIN_PRIVATE_KEY")).unwrap(),
            )
            .unwrap();
            let pem = Secp256k1Identity::from_private_key(private_key);
            pem.unwrap()
        }
        #[cfg(not(feature = "ic-admin"))]
        {
            use ic_agent::identity::BasicIdentity;

            let private_key =
                k256::SecretKey::random(&mut k256::elliptic_curve::rand_core::OsRng).to_bytes();
            BasicIdentity::from_raw_key(private_key.as_slice().try_into().unwrap())
        }
    };

    let ic_admin_agent = ic_agent::Agent::builder()
        .with_identity(ic_admin_identity)
        .with_url("https://ic0.app")
        .build()
        .unwrap();

    let app_state = AppState {
        storj_client: Arc::new(
            StorjInterface::new("https://storj-interface.yral.com".to_string()).unwrap(),
        ),
        //TODO: add OFFCHAIN_EVENTS_API_TOKEN to env variables and use it here
        events_service: EventService::with_auth_token(
            env::var("OFFCHAIN_EVENTS_API_TOKEN").unwrap(),
        ),
        ic_admin_agent: ic_admin_agent,
    };

    let app = Router::new()
        .route("/get-upload-url", get(get_upload_url))
        .route(
            "/update-video-metadata",
            post(api::update_video_metadata::update_video_metadata),
        )
        .with_state(app_state);

    let listner = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();

    axum::serve(listner, app).await.unwrap();
}
