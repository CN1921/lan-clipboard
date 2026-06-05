//! System tray integration (feature-gated)
//!
//! Provides a minimal system tray icon with menu entries to Quit and Show status.
//! When the `tray` feature is not enabled, start_tray returns an error so headless
//! builds remain functional.

#[cfg(feature = "tray")]
mod platform {
    use std::sync::mpsc::{self, Sender};
    use std::thread;
    use tray_item::TrayItem;

    pub fn start_tray_background() -> Result<Sender<()>, Box<dyn std::error::Error>> {
        let (tx, rx) = mpsc::channel::<()>();

        // Spawn the tray in a separate thread to avoid blocking the caller.
        thread::spawn(move || {
            match TrayItem::new("lan-clipboard", "icon-name") {
                Ok(mut tray) => {
                    let quit_tx = tx.clone();

                    // Add a Quit menu item
                    if let Err(e) = tray.add_menu_item("Quit", move || {
                        // send a signal to the main thread to exit
                        let _ = quit_tx.send(());
                    }) {
                        eprintln!("failed to add quit menu item to tray: {}", e);
                    }

                    // Add an About menu item
                    let _ = tray.add_menu_item("About", || {
                        // Could show a window; for now print to stderr
                        eprintln!("lan-clipboard — local network clipboard sync");
                    });

                    // Keep the thread alive while the tray exists and no quit signal
                    // Since tray-item manages its own event loop, just block on rx
                    let _ = rx.recv();
                }
                Err(e) => {
                    eprintln!("failed to create tray item: {}", e);
                }
            }
        });

        Ok(tx)
    }
}

#[cfg(feature = "tray")]
/// Start the system tray. Returns a handle which will send a message when the user requests Quit.
pub fn start_tray() -> Result<(), Box<dyn std::error::Error>> {
    // Starting tray in background and ignoring the sender — the library caller can use
    // other shutdown mechanisms. Keep this function simple.
    let _ = platform::start_tray_background()?;
    Ok(())
}

#[cfg(not(feature = "tray"))]
/// Stub for non-tray builds.
pub fn start_tray() -> Result<(), Box<dyn std::error::Error>> {
    Err("tray feature not enabled".into())
}
