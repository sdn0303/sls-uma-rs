use aws_lambda_events::encodings::Body;
use aws_lambda_events::event::apigw::ApiGatewayProxyResponse;
use aws_lambda_events::http::HeaderMap;

pub fn apigw_response(
    status_code: i64,
    body: Option<Body>,
    headers: Option<HeaderMap>,
) -> ApiGatewayProxyResponse {
    ApiGatewayProxyResponse {
        status_code,
        body,
        headers: headers.unwrap_or_default(),
        ..Default::default()
    }
}
