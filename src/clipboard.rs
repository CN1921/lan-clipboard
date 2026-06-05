//! Clipboard integration (feature-gated)
//!
//! Provides simple read/write helpers. When compiled without the `clipboard` feature,
//! the functions return errors so the library remains usable in headless or CI builds.

#[cfg(feature = "clipboard")]
use clipboard::{ClipboardContext, ClipboardProvider};

/// Read the system clipboard as UTF-8 text.
/// Returns an error if the `clipboard` feature is not enabled or on failure to access the clipboard.
pub fn read_clipboard() -> Result<String, Box<dyn std::error::Error>> {
    read_clipboard_impl()
}

/// Write UTF-8 text to the system clipboard.
/// Returns an error if the `clipboard` feature is not enabled or on failure to write.
pub fn write_clipboard(text: &str) -> Result<(), Box<dyn std::error::Error>> {
    write_clipboard_impl(text)
}

#[cfg(feature = "clipboard")]
fn read_clipboard_impl() -> Result<String, Box<dyn std::error::Error>> {
    let mut ctx: ClipboardContext = ClipboardProvider::new()?;
    let contents = ctx.get_contents()?;
    Ok(contents)
}

#[cfg(not(feature = "clipboard"))]
fn read_clipboard_impl() -> Result<String, Box<dyn std::error::Error>> {
    Err("clipboard feature not enabled".into())
}

#[cfg(feature = "clipboard")]
fn write_clipboard_impl(text: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut ctx: ClipboardContext = ClipboardProvider::new()?;
    ctx.set_contents(text.to_owned())?;
    Ok(())
}

#[cfg(not(feature = "clipboard"))]
fn write_clipboard_impl(_text: &str) -> Result<(), Box<dyn std::error::Error>> {
    Err("clipboard feature not enabled".into())
}

#[cfg(all(test, feature = "clipboard"))]
mod tests {
    use super::*;

    #[test]
    fn test_clipboard_write_read() {
        let txt = format!("test-clipboard-{}", uuid::Uuid::new_v4());
        write_clipboard(&txt).expect("write");
        let got = read_clipboard().expect("read");
        assert_eq!(got, txt);
    }
}
