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
            .show()?;
        let device = rodio::default_output_device().unwrap();

        let file = File::open("/usr/share/sounds/freedesktop/stereo/message-new-instant.oga")?;
        let source = rodio::Decoder::new(BufReader::new(file)).unwrap_or_else(|err| {
            log::info!("{:?}", err);
            panic!();
        });
        rodio::play_raw(&device, source.convert_samples());
        Ok(())
    }
}
