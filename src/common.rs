use idevice::provider::IdeviceProvider;
use idevice::usbmuxd::{UsbmuxdAddr, UsbmuxdConnection};
use std::error::Error;

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum MessageType {
    Video = 1,
}

pub fn calculate_checksum(
    ipv6_header: &etherparse::Ipv6Header,
    udp_header: &etherparse::UdpHeader,
    msg_type: MessageType,
    payload: &[u8],
) -> u16 {
    use etherparse::checksum::Sum16BitWords;
    use etherparse::IpNumber;

    let mut sum = Sum16BitWords::new();
    sum = sum.add_16bytes(ipv6_header.source);
    sum = sum.add_16bytes(ipv6_header.destination);
    sum = sum.add_4bytes((u32::from(ipv6_header.payload_length)).to_be_bytes());
    sum = sum.add_4bytes((u32::from(u8::from(IpNumber::UDP))).to_be_bytes());

    // UDP Header parts (checksum 0)
    sum = sum.add_2bytes(udp_header.source_port.to_be_bytes());
    sum = sum.add_2bytes(udp_header.destination_port.to_be_bytes());
    sum = sum.add_2bytes(udp_header.length.to_be_bytes());

    let msg_type_val = msg_type as u8;

    // Handle payload alignment (MessageType is 1 byte, so shifts payload words)
    if !payload.is_empty() {
        // Combine msg_type and first byte of payload into one word
        sum = sum.add_2bytes([msg_type_val, payload[0]]);
        // Add remainder
        sum = sum.add_slice(&payload[1..]);
    } else {
        sum = sum.add_slice(&[msg_type_val]);
    }

    sum.to_ones_complement_with_no_zero()
}

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
