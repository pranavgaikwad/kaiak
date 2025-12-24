use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, fmt, EnvFilter};

pub mod settings;
pub mod security;
pub mod validation;

/// Initialize logging system with structured output and environment-based level filtering
pub fn init_logging() -> anyhow::Result<()> {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| {
            EnvFilter::new("kaiak=info,tower_lsp=info,goose=info,tokio=warn,h2=warn")
        });

    tracing_subscriber::registry()
        .with(env_filter)
        .with(
            fmt::layer()
                .with_target(true)
                .with_thread_names(false)
                .with_file(true)
                .with_line_number(true)
                .with_level(true)
                .compact(),
        )
        .try_init()?;

    tracing::info!("Structured logging initialized");
    Ok(())
}

/// Initialize logging for testing with reduced verbosity
pub fn init_test_logging() -> anyhow::Result<()> {
    let env_filter = EnvFilter::new("kaiak=debug");

    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt::layer().with_test_writer().compact())
        .try_init()
        .or_else(|_| Ok(())) // Ignore if already initialized
}

/// Create a structured span for operation tracking
#[macro_export]
macro_rules! operation_span {
    ($level:expr, $name:expr, $($field:tt)*) => {
        tracing::span!($level, $name, $($field)*)
    };
}

/// Log structured events with consistent formatting
#[macro_export]
macro_rules! log_event {
    (session = $session_id:expr, $level:ident, $($field:tt)*) => {
        tracing::$level!(
            session_id = $session_id,
            $($field)*
        );
    };
    (request = $request_id:expr, $level:ident, $($field:tt)*) => {
        tracing::$level!(
            request_id = $request_id,
            $($field)*
        );
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_logging_initialization() {
        // Test should not panic
        let _ = init_logging();
    }
}