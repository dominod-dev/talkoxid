use super::core::Notification;
use std::error::Error;

pub struct DesktopNotifier;

impl Notification for DesktopNotifier {
    fn notify(&self, title: &str, content: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
        notify_rust::Notification::new()
            .summary(&title[..])
            .body(&content[..])
            .show()?;
        Ok(())
    }
}
