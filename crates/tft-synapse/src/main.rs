//! TFT Synapse binary entrypoint.
//! Wires all crates together and launches the egui window.

mod args;
mod updater;

use clap::Parser;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use tft_capture::auto_detect_reader;
use tft_ui::{app::AppMessage, TftSynapseApp};
use tracing::{info, warn};

fn main() {
    let args = args::Args::parse();

    let level = match args.log_level.as_str() {
        "trace" => tracing::Level::TRACE,
        "debug" => tracing::Level::DEBUG,
        "warn" => tracing::Level::WARN,
        "error" => tracing::Level::ERROR,
        _ => tracing::Level::INFO,
    };
    tracing_subscriber::fmt().with_max_level(level).init();

    info!("TFT Synapse v{} starting", env!("CARGO_PKG_VERSION"));

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

    // Spawn a background thread to check for updates once at startup.
    {
        let tx = tx.clone();
        thread::spawn(move || {
            if let Some(info) = updater::check_for_update() {
                if info.update_available {
                    let _ = tx.send(AppMessage::UpdateAvailable {
                        version: info.latest_version,
                        url: info.release_url,
                    });
                }
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

    // The app is handed to eframe only once a window and renderer exist, so it is kept
    // here until then: if OpenGL is missing (a VM, a remote desktop, an old driver),
    // the same app is started again on wgpu (Direct3D 12 on Windows, which falls back to
    // the WARP software rasterizer when there is no GPU).
    let app = Rc::new(RefCell::new(Some(app)));
    let mut last_error = None;
    for renderer in [eframe::Renderer::Glow, eframe::Renderer::Wgpu] {
        let native_options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([args.width as f32, args.height as f32])
                .with_title("TFT Synapse")
                .with_resizable(true)
                .with_always_on_top(),
            renderer,
            ..Default::default()
        };
        let slot = Rc::clone(&app);
        let run_result = eframe::run_native(
            "TFT Synapse",
            native_options,
            Box::new(move |_cc| match slot.borrow_mut().take() {
                Some(app) => Ok(Box::new(app) as Box<dyn eframe::App>),
                None => Err("the app was already started".into()),
            }),
        );
        match run_result {
            Ok(()) => return,
            // The app was started and the window then failed: do not start it twice.
            Err(e) if app.borrow().is_none() => {
                eprintln!("UI error: {}", e);
                std::process::exit(1);
            }
            Err(e) => {
                warn!(
                    "{:?} renderer unavailable ({}); trying the next one",
                    renderer, e
                );
                last_error = Some(e);
            }
        }
    }
    if let Some(e) = last_error {
        eprintln!("UI error: {}", e);
    }
    std::process::exit(1);
}
