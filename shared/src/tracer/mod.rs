use crate::utils::env::get_env;

use opentelemetry::trace::TracerProvider;
use opentelemetry::KeyValue;
use opentelemetry_sdk::trace as sdktrace;
use tracing_subscriber::{fmt, layer::SubscriberExt, Registry};

pub fn init_tracing() {
    // 環境変数からサービス情報を取得
    let service_name = get_env("SERVICE_NAME", "local");
    let service_version = get_env("SERVICE_VERSION", "local");
    let service_environment = get_env("SERVICE_ENVIRONMENT", "local");

    let tracer_provider = sdktrace::TracerProvider::builder()
        .with_config(
            sdktrace::Config::default()
                .with_id_generator(opentelemetry_aws::trace::XrayIdGenerator::default())
                .with_resource(opentelemetry_sdk::resource::Resource::new(vec![
                    KeyValue::new("service.name", service_name.clone()),
                    KeyValue::new("service.version", service_version),
                    KeyValue::new("environment", service_environment),
                ])),
        )
        .build();

    let tracer = tracer_provider
        .tracer_builder(service_name.clone())
        .with_version(env!("CARGO_PKG_VERSION"))
        .build();

    let telemetry_layer = tracing_opentelemetry::layer().with_tracer(tracer);
    let subscriber = Registry::default().with(telemetry_layer).with(fmt::layer());
    tracing::subscriber::set_global_default(subscriber).expect("Failed to set tracing subscriber.");

    tracing::info!("Tracing initialized for AWS X‑Ray");
}
