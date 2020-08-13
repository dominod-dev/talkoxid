use super::core::Notification;
use rodio::Source;
use std::error::Error;
use std::fs::File;
use std::io::BufReader;

pub struct DesktopNotifier;

impl Notification for DesktopNotifier {
    fn notify(&self, title: &str, content: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
        notify_rust::Notification::new()
            .summary(&title[..])
            .body(&content[..])
            .timeout(20000)
            .show()?;
        if let Some(device) = rodio::default_output_device() {
            let maybe_file =
                File::open("/usr/share/sounds/freedesktop/stereo/message-new-instant.ogaa");
            if let Ok(file) = maybe_file {
                let maybe_source = rodio::Decoder::new(BufReader::new(file));
                if let Ok(source) = maybe_source {
                    rodio::play_raw(&device, source.convert_samples());
                }
            }
        }

        Ok(())
    }
}
