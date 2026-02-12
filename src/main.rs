use std::sync::Arc;

use axum::{
    Router,
    routing::{get, post},
};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::{
    api::get_upload_url::get_upload_url,
    app_state::AppState,
    utils::{
        events_interface::EventService, notification_client::NotificationClient,
        storj_interface::StorjInterface,
    },
};
#[derive(OpenApi)]
#[openapi(
    paths(
        api::get_upload_url::get_upload_url,
        api::update_video_metadata::update_video_metadata,
        api::mark_post_as_published::mark_post_as_published,
    ),
    components(
        schemas(
            api::get_upload_url::GetUploadUrlReq,
            api::get_upload_url::GetUploadUrlResp,
            api::update_video_metadata::UpdateMetadataRequest,
            api::mark_post_as_published::MarkPostAsPublishedRequest,
            utils::types::DelegatedIdentityWire,
        )
    ),
    tags(
        (name = "video-upload-service", description = "Yral Video Upload Service API")
    )
)]
struct ApiDoc;

use std::env;

pub mod api;
pub mod app_state;
pub mod utils;

fn main() {
    #[cfg(not(feature = "local"))]
    let _guard = {
        let app_env = std::env::var("APP_ENV").unwrap_or_else(|_| "production".to_string());
        Some(sentry::init((
            "https://5f10027ca345020d4382f7acbedeac3e@apm.yral.com/18",
            sentry::ClientOptions {
                release: sentry::release_name!(),
                send_default_pii: true,
                environment: Some(app_env.clone().into()),
                ..Default::default()
            },
        )))
    };

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            env_logger::init();
            let ic_admin_identity = {
                #[cfg(not(feature = "local"))]
                {
                    use ic_agent::identity::Secp256k1Identity;
                    let private_key = env::var("IC_ADMIN_PRIVATE_KEY")
                        .expect("IC_ADMIN_PRIVATE_KEY must be set in environment variables");

                    let pem = Secp256k1Identity::from_pem(stringreader::StringReader::new(
                        private_key.as_str(),
                    ))
                    .unwrap();

                    pem
                }
                #[cfg(feature = "local")]
                {
                    use ic_agent::identity::BasicIdentity;

                    let private_key =
                        k256::SecretKey::random(&mut k256::elliptic_curve::rand_core::OsRng)
                            .to_bytes();
                    BasicIdentity::from_raw_key(private_key.as_slice().try_into().unwrap())
                }
            };

            let ic_admin_agent = ic_agent::Agent::builder()
                .with_identity(ic_admin_identity)
                .with_url("https://ic0.app")
                .build()
                .unwrap();

            let event_service = {
                #[cfg(feature = "local")]
                {
                    EventService::with_auth_token("test".to_owned())
                }
                #[cfg(not(feature = "local"))]
                {
                    EventService::with_auth_token(env::var("OFFCHAIN_EVENTS_API_TOKEN").unwrap())
                }
            };

            let notification_client = {
                #[cfg(feature = "local")]
                {
                    NotificationClient::new("test".to_string())
                }
                #[cfg(not(feature = "local"))]
                {
                    NotificationClient::new(
                        env::var("YRAL_METADATA_NOTIFICATION_SERVICE_API_TOKEN").unwrap(),
                    )
                }
            };

            let app_state = AppState {
                storj_client: Arc::new(
                    StorjInterface::new("https://storj-interface.yral.com".to_string()).unwrap(),
                ),
                events_service: event_service,
                ic_admin_agent: ic_admin_agent,
                notification_client: notification_client,
            };

            let app = Router::new()
                .route("/get-upload-url", get(get_upload_url))
                .route(
                    "/update-video-metadata",
                    post(api::update_video_metadata::update_video_metadata),
                )
                .route(
                    "/mark-post-as-published",
                    post(api::mark_post_as_published::mark_post_as_published),
                )
                .merge(SwaggerUi::new("/swagger").url("/api-doc/openapi.json", ApiDoc::openapi()))
                .with_state(app_state);

            let listner = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();

            axum::serve(listner, app).await.unwrap();
        });
}
