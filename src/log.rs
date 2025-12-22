use anyhow::{Context, Result};
use tracing::trace;
use tracing_subscriber::{fmt, EnvFilter};

/// Initializes the global tracing subscriber.
///
/// This should only be called once in the application's lifetime, typically in `main`.
pub fn init_tracing(log_level: Option<&str>) -> Result<()> {
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

            eprintln!("Warning: invalid log level '{wavalyze_level}'. Falling back to '{DEFAULT_LEVEL}'.");
        }
    }

    // Static directives for dependencies which are also using tracing
    filter = filter.add_directive("egui=warn".parse()?);
    filter = filter.add_directive("eframe=warn".parse()?);

    let subscriber = fmt().with_env_filter(filter).finish();

    tracing::subscriber::set_global_default(subscriber).context("failed to set global tracing subscriber")?;

    trace!("tracing initialized!");
    Ok(())
}
