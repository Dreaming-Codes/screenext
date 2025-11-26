use ashpd::desktop::screencast::{CursorMode, Screencast, SourceType};
use ashpd::desktop::PersistMode;
use gstreamer as gst;
use gstreamer::prelude::*;
use gstreamer_app as gst_app;
use log::info;
use std::error::Error;

pub async fn start_session() -> Result<u32, Box<dyn Error>> {
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

pub fn create_pipeline(node_id: u32) -> Result<(gst::Pipeline, tokio::sync::mpsc::Receiver<gst::Buffer>), Box<dyn Error>> {
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
