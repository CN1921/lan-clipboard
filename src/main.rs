fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Minimal binary entrypoint. Feature-gated behavior is implemented in the library.

    // Install a simple panic hook so panics print to stderr with a message.
    std::panic::set_hook(Box::new(|info| {
        eprintln!("Application panicked: {}", info);
    }));

    // Call into library run() which will start tray/net components according to enabled features.
    if let Err(err) = lan_clipboard::run() {
        eprintln!("Error while running lan-clipboard: {}", err);
        std::process::exit(1);
    }

    Ok(())
}
