use crate::aws::dynamodb::error::DynamoDbError;

use aws_config::{meta::region::RegionProviderChain, Region};
use aws_sdk_dynamodb::{
    operation::{
        delete_item::DeleteItemOutput, get_item::GetItemOutput, put_item::PutItemOutput,
        query::QueryOutput, scan::ScanOutput, update_item::UpdateItemOutput,
    },
    types::AttributeValue,
    Client,
};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::instrument;

#[derive(Clone)]
pub struct DynamoDbClient {
    client: Arc<Client>,
}

impl DynamoDbClient {
    pub async fn new(region_string: String) -> Result<Self, DynamoDbError> {
        let region = Region::new(region_string);
        let region_provider = RegionProviderChain::default_provider().or_else(region);
        let config = aws_config::from_env().region(region_provider).load().await;
        let client = Arc::new(Client::new(&config));
        Ok(DynamoDbClient { client })
    }

    pub async fn generate_attribute_names<K: AsRef<str>, V: AsRef<str>>(
        &self,
        items: &[(K, V)],
    ) -> HashMap<String, String> {
        items
            .iter()
            .map(|(k, v)| (k.as_ref().to_string(), v.as_ref().to_string()))
            .collect()
    }

    pub async fn generate_attribute_values<K: AsRef<str>, V: AsRef<str>>(
        &self,
        items: &[(K, V)],
    ) -> HashMap<String, AttributeValue> {
        items
            .iter()
            .map(|(k, v)| {
                (
                    k.as_ref().to_string(),
                    AttributeValue::S(v.as_ref().to_string()),
                )
            })
            .collect()
    }

    #[instrument(skip(self, key), fields(table = %table_name), name = "aws.dynamodb.get_item")]
    pub async fn get_item(
        &self,
        table_name: &str,
        key: &HashMap<String, AttributeValue>,
    ) -> Result<Option<HashMap<String, AttributeValue>>, DynamoDbError> {
        let result: GetItemOutput = self
            .client
            .get_item()
            .table_name(table_name)
            .set_key(Some(key.clone()))
            .send()
            .await?;

        Ok(result.item)
    }

    #[instrument(skip(self, item), fields(table = %table_name), name = "aws.dynamodb.put_item")]
    pub async fn put_item(
        &self,
        table_name: &str,
        item: HashMap<String, AttributeValue>,
    ) -> Result<PutItemOutput, DynamoDbError> {
        let result: PutItemOutput = self
            .client
            .put_item()
            .table_name(table_name)
            .set_item(Some(item.clone()))
            .send()
            .await?;

        Ok(result)
    }

    #[instrument(
        skip(self, key, expression_attribute_values),
        fields(table = %table_name),
        name = "aws.dynamodb.update_item"
    )]
    pub async fn update_item(
        &self,
        table_name: &str,
        key: &HashMap<String, AttributeValue>,
        update_expression: &str,
        expression_attribute_names: &HashMap<String, String>,
        expression_attribute_values: &HashMap<String, AttributeValue>,
    ) -> Result<UpdateItemOutput, DynamoDbError> {
        let result: UpdateItemOutput = self
            .client
            .update_item()
            .table_name(table_name)
            .set_key(Some(key.clone()))
            .update_expression(update_expression)
            .set_expression_attribute_names(Some(expression_attribute_names.clone()))
            .set_expression_attribute_values(Some(expression_attribute_values.clone()))
            .send()
            .await?;

        Ok(result)
    }

    #[instrument(skip(self, key), fields(table = %table_name), name = "aws.dynamodb.delete_item")]
    pub async fn delete_item(
        &self,
        table_name: &str,
        key: &HashMap<String, AttributeValue>,
    ) -> Result<DeleteItemOutput, DynamoDbError> {
        let result: DeleteItemOutput = self
            .client
            .delete_item()
            .table_name(table_name)
            .set_key(Some(key.clone()))
            .send()
            .await?;

        Ok(result)
    }

    #[instrument(skip(self), fields(table = %table_name), name = "aws.dynamodb.scan_table")]
    pub async fn scan_table(&self, table_name: &str) -> Result<ScanOutput, DynamoDbError> {
        let result: ScanOutput = self.client.scan().table_name(table_name).send().await?;

        Ok(result)
    }

    #[instrument(
        skip(self, expression_attribute_names, expression_attribute_values),
        fields(table = %table_name),
        name = "aws.dynamodb.query_table"
    )]
    pub async fn query_table(
        &self,
        table_name: &str,
        key_condition_expression: &str,
        expression_attribute_names: &HashMap<String, String>,
        expression_attribute_values: &HashMap<String, AttributeValue>,
    ) -> Result<QueryOutput, DynamoDbError> {
        let result: QueryOutput = self
            .client
            .query()
            .table_name(table_name)
            .key_condition_expression(key_condition_expression)
            .set_expression_attribute_names(Some(expression_attribute_names.clone()))
            .set_expression_attribute_values(Some(expression_attribute_values.clone()))
            .send()
            .await?;

        Ok(result)
    }
}
