use anyhow::{Context, Result};
use egui_tracing::EventCollector;
use tracing::trace;
use tracing_subscriber::EnvFilter;
#[cfg(not(target_arch = "wasm32"))]
use tracing_subscriber::fmt;
use tracing_subscriber::prelude::*;

pub type TracingCollector = EventCollector;

/// Initializes the global tracing subscriber.
///
/// This should only be called once in the application's lifetime, typically in `main`.
pub fn init_tracing(log_level: Option<&str>) -> Result<TracingCollector> {
    const DEFAULT_LEVEL: &str = "info";
    let wavalyze_level = log_level.unwrap_or(DEFAULT_LEVEL);

    let mut filter = EnvFilter::from_default_env();

    // Attempt to parse user-provided level, fall back on failure.
    let directive_str = format!("wavalyze={wavalyze_level}");
    match directive_str.parse() {
        Ok(directive) => {
            filter = filter.add_directive(directive);
        }
        Err(_) => {
            // Fallback to default level
            let fallback_str = format!("wavalyze={DEFAULT_LEVEL}");
            let fallback_directive = fallback_str
                .parse()
                .with_context(|| format!("failed to parse fallback directive {fallback_str}"))?;

            filter = filter.add_directive(fallback_directive);

            eprintln!(
                "Warning: invalid log level '{wavalyze_level}'. Falling back to '{DEFAULT_LEVEL}'."
            );
        }
    }

    // Static directives for dependencies which are also using tracing
    filter = filter.add_directive("egui=warn".parse()?);
    filter = filter.add_directive("eframe=warn".parse()?);

    // Parse the level for the collector (case-insensitive, fallback to INFO)
    let collector_level = wavalyze_level
        .to_uppercase()
        .parse::<tracing::Level>()
        .unwrap_or(tracing::Level::INFO);

    let collector = EventCollector::default().with_max_level(collector_level);

    #[cfg(not(target_arch = "wasm32"))]
    let subscriber = tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer())
        .with(collector.clone());

    #[cfg(target_arch = "wasm32")]
    let subscriber = tracing_subscriber::registry()
        .with(filter)
        .with(collector.clone());

    tracing::subscriber::set_global_default(subscriber)
        .context("failed to set global tracing subscriber")?;

    trace!("tracing initialized!");
    Ok(collector)
}
