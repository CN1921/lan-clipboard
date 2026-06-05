fn main() -> Result<(), Box<dyn std::error::Error>> {
    // When building a binary, enable features as needed via cargo features.
    // The library run() handles feature-gated components.
    lan_clipboard::run()?;
    Ok(())
}
