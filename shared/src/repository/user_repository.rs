use crate::aws::dynamodb::client::DynamoDbClient;
use crate::entity::user::User;

use anyhow::{anyhow, Error as AnyhowError, Result};
use async_trait::async_trait;
use tracing::{debug, error};

#[async_trait]
pub trait UserRepository {
    async fn get_user_by_id(&self, user_id: String) -> Result<User, AnyhowError>;
    async fn get_users_by_organization_id(
        &self,
        organization_id: String,
    ) -> Result<Vec<User>, AnyhowError>;
    async fn create_user(&self, user: User) -> Result<User, AnyhowError>;
    async fn delete_user_by_id(
        &self,
        user_id: String,
        organization_id: String,
    ) -> Result<(), AnyhowError>;
    async fn update_user(&self, user: User) -> Result<User, AnyhowError>;

    async fn find_organization_id_by_name(
        &self,
        organization_name: &str,
    ) -> Result<Option<String>, AnyhowError>;
    async fn organization_exists(&self, organization_name: &str) -> Result<bool, AnyhowError>;
    async fn is_first_user_in_organization(
        &self,
        organization_name: &str,
    ) -> Result<bool, AnyhowError>;
}

pub struct UserRepositoryImpl {
    client: DynamoDbClient,
    table_name: String,
}

impl UserRepositoryImpl {
    pub fn new(client: DynamoDbClient, table_name: String) -> Self {
        Self { client, table_name }
    }
}

#[async_trait]
impl UserRepository for UserRepositoryImpl {
    async fn get_user_by_id(&self, user_id: String) -> Result<User, AnyhowError> {
        let key_condition_expression = "#id = :id_value";
        let expression_attribute_names =
            self.client.generate_attribute_names(&[("#id", "id")]).await;
        let expression_attribute_values = self
            .client
            .generate_attribute_values(&[(":id", user_id)])
            .await;

        let opt = self
            .client
            .query_table(
                &self.table_name,
                key_condition_expression,
                &expression_attribute_names,
                &expression_attribute_values,
            )
            .await?;
        match opt.items {
            Some(items) => {
                let user = User::from_item(items.first().expect("user not found in table"))?;
                Ok(user)
            }
            None => {
                error!("No user found in table");
                Err(anyhow!("Unable to get user by id"))
            }
        }
    }

    async fn get_users_by_organization_id(
        &self,
        organization_id: String,
    ) -> Result<Vec<User>, AnyhowError> {
        let key_condition_expression = "#organization_id = :organization_id_value";
        let expression_attribute_names = self
            .client
            .generate_attribute_names(&[("#organization_id", "organization_id")])
            .await;
        let expression_attribute_values = self
            .client
            .generate_attribute_values(&[(":organization_id", organization_id)])
            .await;

        let opt = self
            .client
            .query_table(
                &self.table_name,
                key_condition_expression,
                &expression_attribute_names,
                &expression_attribute_values,
            )
            .await?;

        let items = opt
            .items
            .as_ref()
            .ok_or_else(|| anyhow!("No items found"))?;
        let users: Result<Vec<User>> = items
            .iter()
            .map(move |item| {
                User::from_item(item).map_err(|e| anyhow!("Failed to parse user from item: {}", e))
            })
            .collect();
        let users = users?;

        Ok(users)
    }

    async fn create_user(&self, user: User) -> Result<User, AnyhowError> {
        debug!("Creating user in DynamoDB: {:?}", user);

        let items = self
            .client
            .generate_attribute_values(&[
                ("id", &user.id),
                ("user_name", &user.name),
                ("email", &user.email),
                ("organization_id", &user.organization_id),
                ("organization_name", &user.organization_name),
                ("roles", &user.join_roles()),
            ])
            .await;

        debug!("Generated DynamoDB items: {:?}", items);

        let _ = self
            .client
            .put_item(&self.table_name, items)
            .await
            .map_err(|e| {
                error!("DynamoDB PutItem failed: {:?}", e);
                anyhow!("DynamoDB PutItem failed: {:?}", e)
            })?;

        // PutItem operation doesn't return attributes on success
        // If we reach here, the operation was successful
        debug!("dynamodb put item successful for user: {}", user.id);

        // Return the original user object since PutItem doesn't return the item
        Ok(user)
    }

    async fn delete_user_by_id(
        &self,
        user_id: String,
        organization_id: String,
    ) -> Result<(), AnyhowError> {
        let key = self
            .client
            .generate_attribute_values(&[("id", &user_id), ("organization_id", &organization_id)])
            .await;
        let opt = self.client.delete_item(&self.table_name, &key).await;
        match opt {
            Ok(_) => Ok(()),
            Err(e) => Err(anyhow!("Unable to delete user by id: {:?}", e)),
        }
    }

    async fn update_user(&self, user: User) -> Result<User, AnyhowError> {
        let key = self
            .client
            .generate_attribute_values(&[
                ("id", &user.id),
                ("organization_id", &user.organization_id),
            ])
            .await;
        let update_expression = "SET #email = :email, #user_name = :user_name, #organization_name = :organization_name, #roles = :roles";
        let expression_attribute_names = self
            .client
            .generate_attribute_names(&[
                ("#email", "email"),
                ("#user_name", "user_name"),
                ("#organization_name", "organization_name"),
                ("#roles", "roles"),
            ])
            .await;
        let expression_attribute_values = self
            .client
            .generate_attribute_values(&[
                (":email", &user.email),
                (":user_name", &user.name),
                (":organization_name", &user.organization_name),
                (":roles", &user.join_roles()),
            ])
            .await;
        let output = self
            .client
            .update_item(
                &self.table_name,
                &key,
                update_expression,
                &expression_attribute_names,
                &expression_attribute_values,
            )
            .await?;
        match output.attributes() {
            Some(item) => {
                debug!("dynamodb update item output: {:?}", item);
                let user = User::from_item(item)?;
                Ok(user)
            }
            None => {
                let err_msg = "dynamodb update item failed";
                error!(err_msg);
                Err(anyhow!(err_msg))
            }
        }
    }

    async fn find_organization_id_by_name(
        &self,
        organization_name: &str,
    ) -> Result<Option<String>, AnyhowError> {
        let response = self.client.scan_table(&self.table_name).await?;

        let organization_id = response
            .items
            .as_ref()
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| {
                        item.get("organization_name")
                            .and_then(|attr| attr.as_s().ok())
                            .filter(|&org_name| org_name == organization_name)
                            .and_then(|_| item.get("organization_id"))
                            .and_then(|attr| attr.as_s().ok())
                            .map(|s| s.to_string())
                    })
                    .next()
            })
            .flatten();

        Ok(organization_id)
    }

    async fn organization_exists(&self, organization_name: &str) -> Result<bool, AnyhowError> {
        let response = self.client.scan_table(&self.table_name).await?;

        let exists = response
            .items
            .as_ref()
            .map(|items| {
                items.iter().any(|item| {
                    item.get("organization_name")
                        .and_then(|attr| attr.as_s().ok())
                        .map_or(false, |org_name| org_name == organization_name)
                })
            })
            .unwrap_or(false);

        Ok(exists)
    }

    async fn is_first_user_in_organization(
        &self,
        organization_name: &str,
    ) -> Result<bool, AnyhowError> {
        let response = self.client.scan_table(&self.table_name).await?;

        let has_existing_users = response
            .items
            .as_ref()
            .map(|items| {
                items.iter().any(|item| {
                    item.get("organization_name")
                        .and_then(|attr| attr.as_s().ok())
                        .map_or(false, |org_name| org_name == organization_name)
                })
            })
            .unwrap_or(false);

        Ok(!has_existing_users)
    }
}
