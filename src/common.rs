use idevice::provider::IdeviceProvider;
use idevice::usbmuxd::{UsbmuxdAddr, UsbmuxdConnection};
use std::error::Error;

pub async fn get_provider(
    udid: Option<&String>,
    _host: Option<&String>,
    _pairing_file: Option<&String>,
    label: &str,
) -> Result<Box<dyn IdeviceProvider>, Box<dyn Error>> {
    let mut usbmuxd = UsbmuxdConnection::default().await?;
    let devices = usbmuxd.get_devices().await?;

    if devices.is_empty() {
        return Err("No devices connected!".into());
    }

    let target_device = if let Some(target_udid) = udid {
        devices
            .into_iter()
            .find(|d| d.udid == *target_udid)
            .ok_or("Device with specified UDID not found")?
    } else {
        devices.into_iter().next().ok_or("No devices found")?
    };

    // Create provider for the device using usbmuxd address
    let provider = target_device.to_provider(UsbmuxdAddr::from_env_var()?, label.to_string());
    
    Ok(Box::new(provider))
}
