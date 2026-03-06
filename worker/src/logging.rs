use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, fmt};
use std::env;

pub fn init_logging() -> Result<(), Box<dyn std::error::Error>> {
    let log_level = env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string());
    
    eprintln!("DEBUG: Setting log level to: {}", log_level);
    
    let filter_str = format!("worker={}", log_level);
    eprintln!("DEBUG: Using filter string: {}", filter_str);
    
    let env_filter = EnvFilter::try_from_str(&filter_str)
        .unwrap_or_else(|e| {
            eprintln!("DEBUG: Failed to create filter from '{}', using default: {}", filter_str, e);
            EnvFilter::new("worker=info")
        });

    let is_production = env::var("RUST_ENV").unwrap_or_else(|_| "development".to_string()) == "production";

    if is_production {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(fmt::layer().json().with_current_span(true))
            .init();
    } else {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(fmt::layer()
                .pretty()
                .with_target(true)
                .with_thread_ids(true)
                .with_file(true)
                .with_line_number(true))
            .init();
    }

    tracing::info!("Logging initialized - level: {}, mode: {}", log_level, if is_production { "production" } else { "development" });

    Ok(())
}
