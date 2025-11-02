use tracing::trace;
use tracing_subscriber::{fmt, EnvFilter};

pub fn init_tracing() {
    let filter = EnvFilter::new("wavalyze=trace,egui=warn,mio=warn,tokio=warn");

    let subscriber = fmt()
        .with_env_filter(filter)
        // .with_target(false)
        // .with_thread_ids(true)
        // .with_thread_names(true)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    trace!("tracing initialized!");
}
