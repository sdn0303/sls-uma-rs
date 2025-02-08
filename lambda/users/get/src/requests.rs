use shared::entity::user::User;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub(super) struct ListUsersResponse {
    pub users: Vec<User>,
}
