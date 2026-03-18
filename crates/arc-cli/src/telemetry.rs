use anyhow::{Context, Result};
use std::path::PathBuf;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer};

// Keep the appender guard alive for the duration of the program
pub struct TelemetryGuard {
    _file_guard: tracing_appender::non_blocking::WorkerGuard,
}

impl Drop for TelemetryGuard {
    fn drop(&mut self) {
        // Ensure all OTLP spans are exported on exit
        // global::shutdown_tracer_provider();
    }
}

pub fn init_telemetry(log_dir: PathBuf) -> Result<TelemetryGuard> {
    // Ensure log directory exists
    std::fs::create_dir_all(&log_dir).context("Failed to create log directory")?;

    // 1. File appender for JSON logs
    let file_appender = tracing_appender::rolling::daily(log_dir, "arc.json.log");
    let (non_blocking_appender, file_guard) = tracing_appender::non_blocking(file_appender);

    let file_layer = tracing_subscriber::fmt::layer()
        .json()
        .with_writer(non_blocking_appender)
        .with_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")));

    // 2. Standard interactive CLI formatter (minimal)
    let cli_layer = tracing_subscriber::fmt::layer()
        .with_target(false)
        .with_thread_ids(false)
        .with_level(false)
        .with_filter(EnvFilter::new("warn")); // Keep CLI quiet except warnings

    // 3. OpenTelemetry OTLP Exporter (if enabled)
    /*
    let otel_layer = if std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT").is_ok() {
        global::set_text_map_propagator(TraceContextPropagator::new());

        let exporter = opentelemetry_otlp::new_exporter()
            .tonic()
            .with_env()
            .build_span_exporter()?;

        let provider = opentelemetry_sdk::trace::TracerProvider::builder()
            .with_batch_exporter(exporter, opentelemetry_sdk::runtime::Tokio)
            .with_config(
                opentelemetry_sdk::trace::Config::default()
                    .with_resource(Resource::new(vec![KeyValue::new("service.name", "arc-cli")]))
            )
            .build();

        global::set_tracer_provider(provider.clone());
        let tracer = provider.tracer("arc-cli");

        Some(tracing_opentelemetry::layer().with_tracer(tracer))
    } else {
        None
    */

    // Compose all layers
    let subscriber = tracing_subscriber::registry()
        .with(cli_layer)
        .with(file_layer);

    subscriber.try_init()?;

    Ok(TelemetryGuard {
        _file_guard: file_guard,
    })
}
