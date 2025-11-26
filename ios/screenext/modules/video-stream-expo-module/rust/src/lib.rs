use std::ffi::c_void;
use std::sync::{Arc, Mutex};
use gstreamer::prelude::*;
use gstreamer::{Element, Pipeline, State};
use gstreamer_video::VideoOverlay;
use log::info;
use once_cell::sync::Lazy;

// Global pipeline storage to handle state between FFI calls
static GLOBAL_PIPELINE: Lazy<Mutex<Option<Pipeline>>> = Lazy::new(|| Mutex::new(None));

#[no_mangle]
pub extern "C" fn ios_stream_init() {
    if let Err(e) = gstreamer::init() {
        eprintln!("Failed to initialize GStreamer: {}", e);
    } else {
        println!("GStreamer initialized successfully");
    }
}

#[no_mangle]
pub extern "C" fn ios_stream_start(view_handle: *mut c_void, port: u16) {
    let mut pipeline_guard = GLOBAL_PIPELINE.lock().unwrap();

    if pipeline_guard.is_some() {
        println!("Pipeline already running, stopping first");
        // Stop existing pipeline if any
        if let Some(p) = pipeline_guard.as_ref() {
             let _ = p.set_state(State::Null);
        }
        *pipeline_guard = None;
    }

    println!("Starting stream on port {}", port);

    // Construct the pipeline
    // Note: 'vtdec' is the hardware accelerated H.264 decoder for iOS
    // 'glimagesink' is the OpenGL renderer
    let pipeline_str = format!(
        "udpsrc port={} ! application/x-rtp, media=video, clock-rate=90000, encoding-name=H264, payload=96 ! rtph264depay ! h264parse ! vtdec ! glimagesink name=sink",
        port
    );

    let pipeline = match gstreamer::parse::launch(&pipeline_str) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Failed to parse pipeline: {}", e);
            return;
        }
    };

    let pipeline = match pipeline.downcast::<Pipeline>() {
        Ok(p) => p,
        Err(_) => {
            eprintln!("Failed to downcast to Pipeline");
            return;
        }
    };

    // Setup Video Overlay
    // We need to get the sink element and set the window handle (the UIView)
    if let Some(sink) = pipeline.by_name("sink") {
        // On iOS, GStreamer's glimagesink expects the pointer to the UIView.
        // The UIView MUST be backed by CAEAGLLayer (OpenGL ES)
        unsafe {
             gstreamer_video::VideoOverlay::set_window_handle(&sink, view_handle as usize);
        }
    } else {
        eprintln!("Could not find sink element named 'sink'");
    }

    // Start playing
    if let Err(e) = pipeline.set_state(State::Playing) {
        eprintln!("Failed to set pipeline to Playing: {}", e);
        return;
    }

    println!("Pipeline started!");
    *pipeline_guard = Some(pipeline);
}

#[no_mangle]
pub extern "C" fn ios_stream_stop() {
    let mut pipeline_guard = GLOBAL_PIPELINE.lock().unwrap();
    if let Some(pipeline) = pipeline_guard.take() {
        println!("Stopping pipeline");
        let _ = pipeline.set_state(State::Null);
    }
}
