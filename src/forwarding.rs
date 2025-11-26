use etherparse::{IpNumber, Ipv6FlowLabel, Ipv6Header, PacketHeaders, UdpHeader};
use gstreamer as gst;
use gstreamer::prelude::*;
use idevice::core_device_proxy;
use log::{info, warn, error, trace};
use std::io::IoSlice;
use std::net::Ipv6Addr;
use crate::common::{self, MessageType};

const DEFAULT_HOP_LIMIT: u8 = 64;
const LOG_INTERVAL: u64 = 30;

pub async fn run_packet_loop(
    mut tun_proxy: core_device_proxy::CoreDeviceProxy,
    mut rx: tokio::sync::mpsc::Receiver<gst::Buffer>,
    client_addr: Ipv6Addr,
    server_addr: Ipv6Addr,
    app_port: u16,
    pipeline: gst::Pipeline,
) {
    if let Err(e) = pipeline.set_state(gst::State::Playing) {
        error!("Unable to set the pipeline to the `Playing` state: {}", e);
        return;
    }
    
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
                                         info!("Received UDP from App: {:?} bytes payload", headers.payload.slice().len());
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Received malformed packet, ignoring {}", e)
                    }
                }
            }

            Some(buffer) = rx.recv() => {
                frame_count += 1;

                let map = match buffer.map_readable() {
                    Ok(map) => map,
                    Err(e) => {
                        warn!("Failed to map buffer: {}", e);
                        continue;
                    }
                };
                let payload = map.as_slice();

                // Safety check for IPv6 u16 payload length limit
                let payload_len = payload.len() + std::mem::size_of::<MessageType>();

                if payload_len + UdpHeader::LEN > u16::MAX as usize {
                    warn!("Video frame too large ({}), dropping. Max UDP payload {}.", payload_len, u16::MAX);
                    continue;
                }

                let ipv6_header = Ipv6Header {
                    traffic_class: 0,
                    flow_label: Ipv6FlowLabel::ZERO,
                    payload_length: (UdpHeader::LEN + payload_len) as u16,
                    next_header: IpNumber::UDP,
                    hop_limit: DEFAULT_HOP_LIMIT,
                    source: client_addr.octets(),
                    destination: server_addr.octets(),
                };

                let mut udp_header = UdpHeader {
                    source_port: app_port,
                    destination_port: app_port,
                    length: (UdpHeader::LEN + payload_len) as u16,
                    checksum: 0,
                };

                let msg_type = MessageType::Video;
                udp_header.checksum = common::calculate_checksum(&ipv6_header, &udp_header, msg_type, payload);

                // Serialize headers
                // We have valid checksum now so we can serialize directly
                let mut header_buf = Vec::with_capacity(ipv6_header.header_len() + udp_header.header_len());
                ipv6_header.write(&mut header_buf).unwrap();
                udp_header.write(&mut header_buf).unwrap();

                let msg_slice = [msg_type as u8];
                let bufs = [
                    IoSlice::new(&header_buf),
                    IoSlice::new(&msg_slice),
                    IoSlice::new(payload),
                ];

                if let Err(e) = tun_proxy.send_vectored(&bufs).await {
                    error!("Failed to send packet: {}", e);
                    break;
                }

                if frame_count % LOG_INTERVAL == 0 {
                     trace!("Sent {} video packets (len: {})", frame_count, payload_len);
                }
            }

            else => {
                info!("Channel closed or stream ended.");
                break;
            }
        }
    }

    let _ = pipeline.set_state(gst::State::Null);
}
