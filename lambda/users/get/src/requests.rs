use shared::entity::user::User;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub(super) struct ListUsersResponse {
    pub users: Vec<User>,
}
