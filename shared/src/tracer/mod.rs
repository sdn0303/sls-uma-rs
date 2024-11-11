use crate::utils::env::get_env;

use opentelemetry::trace::TracerProvider;
use opentelemetry::KeyValue;
use opentelemetry_otlp::WithExportConfig;
use tracing_subscriber::{fmt, layer::SubscriberExt, Registry};

pub fn init_tracing() {
    let service_name = get_env("SERVICE_NAME", "local");
    let service_version = get_env("SERVICE_VERSION", "local");
    let service_environment = get_env("SERVICE_ENVIRONMENT", "local");
    let open_telemetry_endpoint = get_env("OTEL_EXPORTER_OTLP_ENDPOINT", "http://localhost:4317");
    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint(open_telemetry_endpoint),
        )
        .with_trace_config(
            opentelemetry_sdk::trace::Config::default()
                .with_sampler(opentelemetry_sdk::trace::Sampler::AlwaysOn)
                .with_id_generator(opentelemetry_aws::trace::XrayIdGenerator::default())
                .with_resource(opentelemetry_sdk::resource::Resource::new(vec![
                    KeyValue::new("service.name", service_name.clone()),
                    KeyValue::new("service.version", service_version),
                    KeyValue::new("environment", service_environment),
                ])),
        )
        .install_batch(opentelemetry_sdk::runtime::Tokio)
        .expect("Failed to install OpenTelemetry tracer.")
        .tracer_builder(service_name)
        .build();

    let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);
    let subscriber = Registry::default().with(telemetry).with(fmt::layer());

    tracing::subscriber::set_global_default(subscriber).expect("Failed to set tracing subscriber.");

    tracing::info!("Tracing has been initialized.");
}
