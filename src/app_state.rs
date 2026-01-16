use std::sync::Arc;

use ic_agent::Agent;

use crate::utils::storj_interface::StorjInterface;

#[derive(Clone)]
pub struct AppState {
    pub storj_client: Arc<StorjInterface>,
    pub ic_admin_agent: Agent,
}
