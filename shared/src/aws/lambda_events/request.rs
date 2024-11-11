use super::response::apigw_response;

use aws_lambda_events::apigw::{ApiGatewayProxyRequest, ApiGatewayProxyResponse};
use lambda_runtime::{Error, LambdaEvent};
use std::future::Future;
use tracing::{info, instrument};

pub struct LambdaEventRequestHandler {}

impl LambdaEventRequestHandler {
    #[instrument(
        skip(event),
        name = "aws.lambda_events.request.get_ids_from_request_context"
    )]
    pub async fn get_ids_from_request_context(
        event: LambdaEvent<ApiGatewayProxyRequest>,
    ) -> Result<(String, String), Error> {
        let headers = event.clone().payload.headers;
        let user_id = headers.get("user_id").expect("missing user id").to_str()?;
        let organization_id = headers
            .get("organization_id")
            .expect("missing organization id")
            .to_str()?;
        Ok((user_id.to_string(), organization_id.to_string()))
    }

    #[instrument(
        skip(event, handler),
        name = "aws.lambda_events.request.handle_requests"
    )]

    pub async fn handle_requests<F, Fut>(
        event: LambdaEvent<ApiGatewayProxyRequest>,
        target: &str,
        handler: F,
    ) -> Result<ApiGatewayProxyResponse, Error>
    where
        F: Fn(LambdaEvent<ApiGatewayProxyRequest>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<ApiGatewayProxyResponse, Error>> + Send,
    {
        let path = event.clone().payload.path.unwrap_or_default();
        match event.clone().payload.resource.as_deref() {
            Some(p) if p == target => {
                info!("Received request for {}", p);
                handler(event).await
            }
            _ => {
                info!("Invalid path: {}", path);
                Ok(apigw_response(404, Some("Not Found".into()), None))
            }
        }
    }
}
