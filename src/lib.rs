pub mod crypto;

#[cfg(feature = "net")]
pub mod net;

#[cfg(feature = "clipboard")]
pub mod clipboard;

#[cfg(feature = "tray")]
pub mod tray;

/// Run the application. This starts optional components based on enabled features.
///
/// Behavior:
/// - If the `tray` feature is enabled, start the system tray (non-blocking).
/// - If the `net` feature is enabled, start the tokio runtime and discovery task and
///   block until the process receives CTRL-C.
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    // Prefer the application's logger if configured by the caller.
    log::info!("starting lan-clipboard");

    #[cfg(feature = "tray")]
    {
        // start_tray returns quickly and runs in background
        crate::tray::start_tray()?;
        log::info!("tray started");
    }

    #[cfg(feature = "net")]
    {
        // Build a runtime and spawn the network discovery which runs until process exit.
        let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build()?;
        rt.spawn(async move {
            if let Err(e) = crate::net::start_discovery().await {
                log::error!("network discovery failed: {}", e);
            }
        });

        log::info!("network discovery running; awaiting CTRL-C to exit");
        // Block on Ctrl-C inside the runtime so the process stays alive while background tasks run.
        rt.block_on(async {
            tokio::signal::ctrl_c().await?;
            Ok::<(), Box<dyn std::error::Error>>(())
        })?;
    }

    // If net feature not enabled, return immediately. Caller (binary) may choose to stay alive.
    Ok(())
}
