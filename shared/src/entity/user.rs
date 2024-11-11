use anyhow::{anyhow, Error};
use aws_sdk_dynamodb::types::AttributeValue;
use bitflags::bitflags;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

bitflags! {
    #[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
    pub struct Permissions: u32 {
        const READ    = 0b0001;
        const WRITE   = 0b0010;
        const CREATE  = 0b0100;
        const DELETE  = 0b1000;
        const UPDATE = 0b1_0000;
    }
}

impl std::fmt::Display for Permissions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut perms = Vec::new();
        if self.contains(Permissions::READ) {
            perms.push("READ");
        }
        if self.contains(Permissions::WRITE) {
            perms.push("WRITE");
        }
        if self.contains(Permissions::CREATE) {
            perms.push("CREATE");
        }
        if self.contains(Permissions::DELETE) {
            perms.push("DELETE");
        }
        if self.contains(Permissions::UPDATE) {
            perms.push("UPDATE");
        }
        write!(f, "{}", perms.join(", "))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Role {
    Admin,
    Reader,
    Writer,
}

impl Role {
    pub fn permissions(&self) -> Permissions {
        match self {
            Role::Admin => {
                Permissions::READ
                    | Permissions::WRITE
                    | Permissions::CREATE
                    | Permissions::DELETE
                    | Permissions::UPDATE
            }
            Role::Reader => Permissions::READ,
            Role::Writer => Permissions::READ | Permissions::WRITE | Permissions::CREATE,
        }
    }
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let role_str = match self {
            Role::Admin => "Admin",
            Role::Reader => "Reader",
            Role::Writer => "Writer",
        };
        write!(f, "{}", role_str)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub name: String,
    pub email: String,
    pub organization_id: String,
    pub organization_name: String,
    pub roles: HashSet<Role>,
}

impl User {
    pub fn new(
        id: String,
        name: String,
        email: String,
        organization_id: String,
        organization_name: String,
        roles: HashSet<Role>,
    ) -> Self {
        User {
            id,
            name,
            email,
            organization_id,
            organization_name,
            roles,
        }
    }

    pub fn permissions(&self) -> Permissions {
        self.roles
            .iter()
            .fold(Permissions::empty(), |acc, role| acc | role.permissions())
    }

    pub fn has_permission(&self, permission: Permissions) -> bool {
        self.permissions().contains(permission)
    }

    pub fn add_role(&mut self, role: Role) {
        if !self.has_role(role) {
            self.roles.insert(role);
        }
    }

    pub fn set_from_roles(&mut self, roles: Vec<Role>) {
        roles.into_iter().for_each(move |role| {
            self.add_role(role);
        });
    }

    pub fn remove_role(&mut self, role: Role) {
        self.roles.remove(&role);
    }

    pub fn has_role(&self, role: Role) -> bool {
        self.roles.contains(&role)
    }

    pub fn roles(&self) -> Vec<Role> {
        self.roles.iter().cloned().collect()
    }

    pub fn join_roles(&self) -> String {
        self.roles
            .iter()
            .map(|role| role.to_string())
            .collect::<Vec<String>>()
            .join(":")
    }

    pub fn from_item(item: &HashMap<String, AttributeValue>) -> Result<User, Error> {
        let id = item
            .get("id")
            .and_then(|v| v.as_s().ok())
            .ok_or_else(|| anyhow!("Missing or invalid 'id' attribute".to_string()))?
            .to_string();

        let name = item
            .get("name")
            .and_then(|v| v.as_s().ok())
            .ok_or_else(|| anyhow!("Missing or invalid 'name' attribute".to_string()))?
            .to_string();

        let email = item
            .get("email")
            .and_then(|v| v.as_s().ok())
            .ok_or_else(|| anyhow!("Missing or invalid 'email' attribute".to_string()))?
            .to_string();

        let organization_id = item
            .get("organization_id")
            .and_then(|v| v.as_s().ok())
            .ok_or_else(|| anyhow!("Missing or invalid 'organization_id' attribute".to_string(),))?
            .to_string();

        let organization_name = item
            .get("organization_name")
            .and_then(|v| v.as_s().ok())
            .ok_or_else(
                || anyhow!("Missing or invalid 'organization_name' attribute".to_string(),),
            )?
            .to_string();

        // 'roles' 属性を取得し、HashSet<Role>に変換
        let roles_attr = item
            .get("roles")
            .and_then(|v| v.as_s().ok())
            .ok_or_else(|| anyhow!("Missing or invalid 'roles' attribute".to_string()))?;

        let mut roles = HashSet::new();
        for role_str in roles_attr.split(':') {
            let role = match role_str.trim() {
                "Admin" => Role::Admin,
                "Reader" => Role::Reader,
                "Writer" => Role::Writer,
                other => {
                    return Err(anyhow!("Unknown role: {}", other));
                }
            };
            roles.insert(role);
        }

        Ok(User {
            id,
            name,
            email,
            organization_id,
            organization_name,
            roles,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_user_permissions() {
        let mut roles = HashSet::new();
        roles.insert(Role::Reader);
        roles.insert(Role::Writer);

        let user = User::new(
            "2".to_string(),
            "Bob".to_string(),
            "bob@example.com".to_string(),
            "org_456".to_string(),
            "ExampleOrg".to_string(),
            roles,
        );

        let permissions = user.permissions();
        assert!(permissions.contains(Permissions::READ));
        assert!(permissions.contains(Permissions::WRITE));
        assert!(permissions.contains(Permissions::CREATE));
        assert!(!permissions.contains(Permissions::DELETE));
    }

    #[tokio::test]
    async fn test_add_remove_role() {
        let mut roles = HashSet::new();
        roles.insert(Role::Reader);

        let mut user = User::new(
            "3".to_string(),
            "Charlie".to_string(),
            "charlie@example.com".to_string(),
            "org_789".to_string(),
            "ExampleOrg".to_string(),
            roles,
        );

        assert!(user.has_role(Role::Reader));
        assert!(user.has_permission(Permissions::READ));
        assert!(!user.has_permission(Permissions::WRITE));

        user.remove_role(Role::Reader);
        assert!(!user.has_role(Role::Reader));
        assert!(!user.has_permission(Permissions::READ));

        user.add_role(Role::Writer);
        assert!(user.has_role(Role::Writer));
        assert!(user.has_permission(Permissions::WRITE));
        assert!(user.has_permission(Permissions::CREATE));

        user.remove_role(Role::Writer);
        assert!(!user.has_role(Role::Writer));
        assert!(!user.has_permission(Permissions::READ));
        assert!(!user.has_permission(Permissions::WRITE));
        assert!(!user.has_permission(Permissions::CREATE));
    }

    #[tokio::test]
    async fn test_user_roles() {
        let mut roles = HashSet::new();
        roles.insert(Role::Reader);
        roles.insert(Role::Writer);

        let user = User::new(
            "4".to_string(),
            "Alice".to_string(),
            "alice@example.com".to_string(),
            "org_123".to_string(),
            "ExampleOrg".to_string(),
            roles,
        );

        let roles = user.roles();
        assert!(roles.contains(&Role::Reader));
        assert!(roles.contains(&Role::Writer));
    }

    #[tokio::test]
    async fn test_role_permissions() {
        assert_eq!(
            Role::Admin.permissions(),
            Permissions::READ
                | Permissions::WRITE
                | Permissions::CREATE
                | Permissions::DELETE
                | Permissions::UPDATE
        );
        assert_eq!(Role::Reader.permissions(), Permissions::READ);
        assert_eq!(
            Role::Writer.permissions(),
            Permissions::READ | Permissions::WRITE | Permissions::CREATE
        );
    }
}
