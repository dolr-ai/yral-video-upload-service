use std::sync::Arc;

use ic_agent::Agent;

use crate::utils::{
    events_interface::EventService, notification_client::NotificationClient,
    storj_interface::StorjInterface,
};

#[derive(Clone)]
pub struct AppState {
    pub storj_client: Arc<StorjInterface>,
    pub ic_admin_agent: Agent,
    pub events_service: EventService,
}
