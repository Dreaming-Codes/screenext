use clap::Parser;
use etherparse::{IpNumber, Ipv6FlowLabel, Ipv6Header, PacketHeaders, UdpHeader};
use idevice::{
    core_device_proxy::{self},
    IdeviceService,
};
use log::{error, info, trace, warn};
use std::io::IoSlice;
use std::net::Ipv6Addr;
use std::error::Error;

// GStreamer imports
use gstreamer as gst;
use gstreamer::prelude::*;
use gstreamer_app as gst_app;

mod common;

use common::MessageType;

// XDG Portal / Pipewire imports
use ashpd::desktop::screencast::{CursorMode, Screencast, SourceType};
use ashpd::desktop::PersistMode;

const DEFAULT_HOP_LIMIT: u8 = 64;
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

async fn connect_to_device(
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

fn extract_handshake_addresses(tun_proxy: &core_device_proxy::CoreDeviceProxy) -> Result<(Ipv6Addr, Ipv6Addr), Box<dyn Error>> {
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

async fn start_screencast_session() -> Result<u32, Box<dyn Error>> {
    info!("Requesting screencast session via XDG Portal...");
    let proxy = Screencast::new().await?;
    let session = proxy.create_session().await?;

    proxy.select_sources(
        &session,
        CursorMode::Embedded,
        SourceType::Monitor | SourceType::Window,
        false, // multiple
        None,
        PersistMode::DoNot,
    ).await?;

    let response = proxy.start(&session, None).await?.response()?;
    
    let stream = response.streams().first().ok_or("No streams returned by portal")?;
    let node_id = stream.pipe_wire_node_id();
    
    Ok(node_id)
}

fn create_pipeline(node_id: u32) -> Result<(gst::Pipeline, tokio::sync::mpsc::Receiver<gst::Buffer>), Box<dyn Error>> {
    // Create GStreamer pipeline
    // We use zerolatency and ultrafast to minimize delay/load.
    // Note: This produces raw H.264 stream chunks.
    let pipeline_str = format!(
        "pipewiresrc path={} ! queue ! videoconvert ! x264enc tune=zerolatency speed-preset=ultrafast ! rtph264pay ! appsink name=sink sync=false",
        node_id
    );

    let pipeline = gst::parse::launch(&pipeline_str)?;

    let pipeline = pipeline
        .downcast::<gst::Pipeline>()
        .map_err(|_| "Expected a pipeline")?;

    let appsink = pipeline
        .by_name("sink")
        .ok_or("Sink not found")?
        .downcast::<gst_app::AppSink>()
        .map_err(|_| "Sink is not an AppSink")?;

    // Channel to send video buffers from GST thread to Tokio thread
    // Capacity 10 to avoid growing too much backlog
    let (tx, rx) = tokio::sync::mpsc::channel::<gst::Buffer>(10);

    appsink.set_callbacks(
        gst_app::AppSinkCallbacks::builder()
            .new_sample(move |appsink| {
                let sample = appsink.pull_sample().map_err(|_| gst::FlowError::Eos)?;
                let buffer = sample.buffer().ok_or(gst::FlowError::Error)?;

                // Cheap ref-count clone to send to the other thread
                let buffer = buffer.to_owned();

                // blocking_send is okay here because we are in the GStreamer streaming thread
                if let Err(_) = tx.blocking_send(buffer) {
                    return Err(gst::FlowError::Eos);
                }

                Ok(gst::FlowSuccess::Ok)
            })
            .build(),
    );

    Ok((pipeline, rx))
}

async fn run_packet_loop(
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

#[tokio::main]
async fn main() {
    env_logger::init();

    // Initialize GStreamer
    if let Err(e) = gst::init() {
        error!("Failed to initialize GStreamer: {}", e);
        return;
    }

    let Args {
        udid,
        host,
        pairing_file,
        app_port,
    } = Args::parse();

    let tun_proxy = match connect_to_device(udid.as_ref(), host.as_ref(), pairing_file.as_ref()).await {
        Ok(p) => p,
        Err(e) => {
            error!("Failed to connect to device: {}", e);
            return;
        }
    };

    let (client_addr, server_addr) = match extract_handshake_addresses(&tun_proxy) {
        Ok(addrs) => addrs,
        Err(e) => {
            error!("Failed to parse handshake addresses: {}", e);
            return;
        }
    };

    info!("-----------------------------");
    info!("UDP Tunnel Established");
    info!("My IP (Host): {}", client_addr);
    info!("Device IP (iOS): {}", server_addr);
    info!("Target App Port: {}", app_port);
    
    // Request Screencast
    let node_id = match start_screencast_session().await {
        Ok(id) => id,
        Err(e) => {
             error!("Screencast setup failed: {}", e);
             return;
        }
    };
    
    info!("Screencast session started. Node ID: {}", node_id);
    info!("Initializing GStreamer pipeline...");
    info!("-----------------------------");

    let (pipeline, rx) = match create_pipeline(node_id) {
        Ok(res) => res,
        Err(e) => {
            error!("Failed to create pipeline: {}", e);
            return;
        }
    };

    run_packet_loop(tun_proxy, rx, client_addr, server_addr, app_port, pipeline).await;
}
