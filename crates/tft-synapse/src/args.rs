use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    name = "tft-synapse",
    about = "AI-powered TFT advisor - real-time augment recommendations and auto-play",
    version = "0.2.0"
)]
pub struct Args {
    /// Run as an always-on-top overlay (transparent window over TFT)
    #[arg(long, default_value_t = false)]
    pub overlay: bool,

    /// Manual input mode (no screen capture or Live API)
    #[arg(long, default_value_t = false)]
    pub manual: bool,

    /// Path to the model weights file
    #[arg(long, default_value = "")]
    pub model_path: String,

    /// Log level (trace, debug, info, warn, error)
    #[arg(long, default_value = "info")]
    pub log_level: String,

    /// Window width in pixels
    #[arg(long, default_value_t = 500)]
    pub width: u32,

    /// Window height in pixels
    #[arg(long, default_value_t = 600)]
    pub height: u32,
}

impl Args {
    pub fn effective_model_path(&self) -> std::path::PathBuf {
        let path = if self.model_path.is_empty() {
            let home = std::env::var("USERPROFILE")
                .or_else(|_| std::env::var("HOME"))
                .unwrap_or_else(|_| ".".to_string());
            std::path::PathBuf::from(home)
                .join(".tft-synapse")
                .join("model.json")
        } else {
            std::path::PathBuf::from(&self.model_path)
        };

        // Ensure the parent directory exists so the model can be saved on first run.
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                if let Err(e) = std::fs::create_dir_all(parent) {
                    eprintln!("warn: could not create model directory {:?}: {}", parent, e);
                }
            }
        }

        path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_model_path_is_in_home() {
        let args = Args {
            overlay: false,
            manual: false,
            model_path: "".to_string(),
            log_level: "info".to_string(),
            width: 500,
            height: 600,
        };
        let path = args.effective_model_path();
        let path_str = path.to_string_lossy();
        assert!(
            path_str.contains(".tft-synapse"),
            "expected .tft-synapse in path: {}",
            path_str
        );
    }

    #[test]
    fn test_custom_model_path_used() {
        let args = Args {
            overlay: false,
            manual: false,
            model_path: "/custom/path/model.json".to_string(),
            log_level: "info".to_string(),
            width: 500,
            height: 600,
        };
        let path = args.effective_model_path();
        assert_eq!(path, std::path::PathBuf::from("/custom/path/model.json"));
    }

    #[test]
    fn test_args_have_reasonable_defaults() {
        let args = Args {
            overlay: false,
            manual: false,
            model_path: "".to_string(),
            log_level: "info".to_string(),
            width: 500,
            height: 600,
        };
        assert_eq!(args.width, 500);
        assert_eq!(args.height, 600);
        assert!(!args.overlay);
    }
}
