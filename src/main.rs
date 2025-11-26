use clap::Parser;
use etherparse::{IpNumber, Ipv6FlowLabel, Ipv6Header, PacketHeaders, UdpHeader};
use idevice::{
    core_device_proxy::{self},
    IdeviceService,
};
use std::net::Ipv6Addr;
use std::time::Duration;
use tokio::time;

mod common;

const DEFAULT_HOP_LIMIT: u8 = 64;
const FRAME_INTERVAL_MS: u64 = 33;
const LOG_INTERVAL: u64 = 30;

#[derive(Parser)]
#[command(version, about)]
struct Args {
    /// IP address of the device
    #[arg(short = 'H', long, value_name = "HOST")]
    host: Option<String>,
    #[arg(short, long, value_name = "PAIRING_FILE")]
    pairing_file: Option<String>,
    #[arg(short, long, value_name = "UDID")]
    udid: Option<String>,
    #[arg(short, long, value_name = "PORT", default_value_t = 12345)]
    app_port: u16,
}

#[tokio::main]
async fn main() {
    env_logger::init();
    let Args {
        udid,
        host,
        pairing_file,
        app_port,
    } = Args::parse();

    let provider = match common::get_provider(
        udid.as_ref(),
        host.as_ref(),
        pairing_file.as_ref(),
        "core_device_proxy",
    )
    .await
    {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{e}");
            return;
        }
    };

    let mut tun_proxy = core_device_proxy::CoreDeviceProxy::connect(&*provider)
        .await
        .expect("Unable to connect");

    // Addresses from the handshake
    // client_addr: The IP 'we' are assigned (the host side)
    // server_addr: The IP the iOS device uses
    let client_addr: Ipv6Addr = tun_proxy
        .handshake
        .client_parameters
        .address
        .parse()
        .expect("Failed to parse client address");
    let server_addr: Ipv6Addr = tun_proxy
        .handshake
        .server_address
        .parse()
        .expect("Failed to parse server address");

    println!("-----------------------------");
    println!("Manual UDP Tunnel Established");
    println!("My IP (Host): {}", client_addr);
    println!("Device IP (iOS): {}", server_addr);
    println!("Target App Port: {}", app_port);
    println!("Sending dummy video frames...");
    println!("-----------------------------");

    // Simulate a video stream interval (30fps = ~33ms)
    let mut interval = time::interval(Duration::from_millis(FRAME_INTERVAL_MS));
    let mut frame_count = 0u64;

    loop {
        tokio::select! {
            Ok(packet) = tun_proxy.recv() => {
                match PacketHeaders::from_ip_slice(&packet) {
                    Ok(headers) => {
                        if let Some(transport) = headers.transport {
                            match transport {
                                etherparse::TransportHeader::Udp(udp) => {
                                    if udp.destination_port == app_port {
                                         println!("Received UDP from App: {:?} bytes payload", headers.payload.slice().len());
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Received malformed packet, ignoring {}", e)
                        // Ignore malformed packets
                    }
                }
            }

            _ = interval.tick() => {
                frame_count += 1;
                let payload = format!("Video Frame #{}", frame_count).into_bytes();

                let ipv6_header = Ipv6Header {
                    traffic_class: 0,
                    flow_label: Ipv6FlowLabel::ZERO,
                    payload_length: (UdpHeader::LEN + payload.len()) as u16,
                    next_header: IpNumber::UDP,
                    hop_limit: DEFAULT_HOP_LIMIT,
                    source: client_addr.octets(),
                    destination: server_addr.octets(),
                };

                let mut udp_header = UdpHeader {
                    source_port: app_port,
                    destination_port: app_port,
                    length: (UdpHeader::LEN + payload.len()) as u16,
                    checksum: 0,
                };

                udp_header.checksum = udp_header.calc_checksum_ipv6(
                    &ipv6_header,
                    &payload
                ).expect("Checksum calculation failed");

                let mut packet_buf = Vec::with_capacity(ipv6_header.header_len() + udp_header.header_len() + payload.len());
                ipv6_header.write(&mut packet_buf).unwrap();
                udp_header.write(&mut packet_buf).unwrap();
                packet_buf.extend_from_slice(&payload);

                if let Err(e) = tun_proxy.send(&packet_buf).await {
                    eprintln!("Failed to send packet: {}", e);
                    break;
                }

                if frame_count % LOG_INTERVAL == 0 {
                     println!("Sent {} frames...", frame_count);
                }
            }
        }
    }
}
