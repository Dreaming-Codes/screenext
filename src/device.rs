use idevice::{
    core_device_proxy::{self},
    IdeviceService,
};
use std::error::Error;
use std::net::Ipv6Addr;
use crate::common;

pub async fn connect(
    udid: Option<&String>,
    host: Option<&String>,
    pairing_file: Option<&String>
) -> Result<core_device_proxy::CoreDeviceProxy, Box<dyn Error>> {
    let provider = common::get_provider(
        udid,
        host,
        pairing_file,
        "core_device_proxy",
    ).await?;

    let tun_proxy = core_device_proxy::CoreDeviceProxy::connect(&*provider).await?;
    Ok(tun_proxy)
}

pub fn extract_handshake_addresses(tun_proxy: &core_device_proxy::CoreDeviceProxy) -> Result<(Ipv6Addr, Ipv6Addr), Box<dyn Error>> {
    // client_addr: The IP 'we' are assigned (the host side)
    // server_addr: The IP the iOS device uses
    let client_addr: Ipv6Addr = tun_proxy
        .handshake
        .client_parameters
        .address
        .parse()?;
        
    let server_addr: Ipv6Addr = tun_proxy
        .handshake
        .server_address
        .parse()?;
        
    Ok((client_addr, server_addr))
}
