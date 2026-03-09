//! TFT Synapse binary entrypoint.
//! Wires all crates together and launches the egui window.

mod args;

use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use clap::Parser;
use tracing::{info, warn};
use tft_capture::auto_detect_reader;
use tft_ui::{TftSynapseApp, app::AppMessage};

fn main() {
    let args = args::Args::parse();

    let level = match args.log_level.as_str() {
        "trace" => tracing::Level::TRACE,
        "debug" => tracing::Level::DEBUG,
        "warn"  => tracing::Level::WARN,
        "error" => tracing::Level::ERROR,
        _       => tracing::Level::INFO,
    };
    tracing_subscriber::fmt()
        .with_max_level(level)
        .init();

    info!("TFT Synapse v0.2.0 starting");

    let model_path = args.effective_model_path();
    info!("Model path: {:?}", model_path);

    let (tx, rx) = mpsc::channel::<AppMessage>();

    {
        let tx = tx.clone();
        thread::spawn(move || {
            let reader = auto_detect_reader();
            info!("Reader mode: {:?}", reader.mode());

            loop {
                match reader.poll() {
                    Ok(Some(state)) => {
                        if tx.send(AppMessage::GameStateUpdate(state)).is_err() {
                            break;
                        }
                    }
                    Ok(None) => {
                        if tx.send(AppMessage::Disconnected).is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        warn!("Reader error: {}", e);
                        if tx.send(AppMessage::Error(e.to_string())).is_err() {
                            break;
                        }
                    }
                }
                thread::sleep(Duration::from_millis(500));
            }
        });
    }

    // tx is moved into spawn above via clone; drop the original so the channel
    // closes when the background thread exits.
    drop(tx);

    let app = match TftSynapseApp::new(model_path, rx) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("Failed to initialize TFT Synapse: {}", e);
            std::process::exit(1);
        }
    };

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([args.width as f32, args.height as f32])
            .with_title("TFT Synapse")
            .with_resizable(true)
            .with_always_on_top(),
        ..Default::default()
    };

    let run_result = eframe::run_native(
        "TFT Synapse",
        native_options,
        Box::new(|_cc| Ok(Box::new(app))),
    );

    if let Err(e) = run_result {
        eprintln!("UI error: {}", e);
        std::process::exit(1);
    }
}
