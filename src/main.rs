use clap::{Arg, Command};
use etherparse::{Ipv6FlowLabel, Ipv6Header, PacketHeaders, UdpHeader};
use idevice::{
    core_device_proxy::{self},
    IdeviceService,
};
use std::net::Ipv6Addr;
use std::time::Duration;
use tokio::time;

mod common;

#[tokio::main]
async fn main() {
    env_logger::init();
    let matches = Command::new("core_device_proxy_tun")
        .about("Start a tunnel")
        .arg(
            Arg::new("host")
                .long("host")
                .value_name("HOST")
                .help("IP address of the device"),
        )
        .arg(
            Arg::new("pairing_file")
                .long("pairing-file")
                .value_name("PATH")
                .help("Path to the pairing file"),
        )
        .arg(
            Arg::new("udid")
                .value_name("UDID")
                .help("UDID of the device (overrides host/pairing file)")
                .index(1),
        )
        .arg(
            Arg::new("app-port")
                .long("port")
                .value_name("PORT")
                .help("UDP port the iOS app is listening on")
                .default_value("12345"),
        )
        .get_matches();

    let udid = matches.get_one::<String>("udid");
    let host = matches.get_one::<String>("host");
    let pairing_file = matches.get_one::<String>("pairing_file");
    let app_port: u16 = matches
        .get_one::<String>("app-port")
        .unwrap()
        .parse()
        .expect("Invalid port");

    let provider = match common::get_provider(udid, host, pairing_file, "core_device_proxy").await {
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
    let mut interval = time::interval(Duration::from_millis(33));
    let mut frame_count = 0u64;

    loop {
        tokio::select! {
            // READ from Device
            Ok(packet) = tun_proxy.recv() => {
                // Parse the raw packet to see if it's interesting
                match PacketHeaders::from_ip_slice(&packet) {
                    Ok(headers) => {
                        if let Some(transport) = headers.transport {
                            match transport {
                                etherparse::TransportHeader::Udp(udp) => {
                                    if udp.destination_port == 12345 { // Assuming we listen on 12345 too
                                         println!("Received UDP from App: {:?} bytes payload", headers.payload.slice().len());
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    Err(_) => {
                        // Ignore malformed packets
                    }
                }
            }

            // WRITE to Device (Simulate App Logic)
            _ = interval.tick() => {
                frame_count += 1;
                let payload = format!("Video Frame #{}", frame_count).into_bytes();

                // 1. Construct IPv6 Header
                let header = Ipv6Header {
                    traffic_class: 0,
                    flow_label: Ipv6FlowLabel::ZERO,
                    payload_length: (8 + payload.len()) as u16, // UDP Header (8) + Payload
                    next_header: etherparse::IpNumber(17),
                    hop_limit: 64,
                    source: client_addr.octets(),
                    destination: server_addr.octets(),
                };

                // 2. Construct UDP Header
                let mut udp_header = UdpHeader {
                    source_port: 12345, // Our fake sender port
                    destination_port: app_port,
                    length: (8 + payload.len()) as u16,
                    checksum: 0,
                };

                // Calculate UDP Checksum (Crucial for iOS to accept it)
                udp_header.checksum = udp_header.calc_checksum_ipv6(
                    &header,
                    &payload
                ).expect("Checksum calculation failed");

                // 3. Serialize to buffer
                let mut packet_buf = Vec::with_capacity(header.header_len() + udp_header.header_len() + payload.len());
                header.write(&mut packet_buf).unwrap();
                udp_header.write(&mut packet_buf).unwrap();
                packet_buf.extend_from_slice(&payload);

                // 4. Send
                if let Err(e) = tun_proxy.send(&packet_buf).await {
                    eprintln!("Failed to send packet: {}", e);
                    break;
                }

                if frame_count % 30 == 0 {
                     println!("Sent {} frames...", frame_count);
                }
            }
        }
    }
}
