mod cli;
mod common;
mod device;
mod forwarding;
mod screencast;

use clap::Parser;
use log::{error, info};
use gstreamer as gst;
use cli::Args;

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

    let tun_proxy = match device::connect(udid.as_ref(), host.as_ref(), pairing_file.as_ref()).await {
        Ok(p) => p,
        Err(e) => {
            error!("Failed to connect to device: {}", e);
            return;
        }
    };

    let (client_addr, server_addr) = match device::extract_handshake_addresses(&tun_proxy) {
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
    let node_id = match screencast::start_session().await {
        Ok(id) => id,
        Err(e) => {
             error!("Screencast setup failed: {}", e);
             return;
        }
    };
    
    info!("Screencast session started. Node ID: {}", node_id);
    info!("Initializing GStreamer pipeline...");
    info!("-----------------------------");

    let (pipeline, rx) = match screencast::create_pipeline(node_id) {
        Ok(res) => res,
        Err(e) => {
            error!("Failed to create pipeline: {}", e);
            return;
        }
    };

    forwarding::run_packet_loop(tun_proxy, rx, client_addr, server_addr, app_port, pipeline).await;
}
