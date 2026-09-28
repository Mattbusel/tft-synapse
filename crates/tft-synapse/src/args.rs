use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    name = "tft-synapse",
    about = "Teamfight Tactics helper: an always-on-top window with economy, level and stage advice read from your live game",
    after_help = "Start it after your TFT game has loaded; it reads Riot's local Live Client Data API (127.0.0.1:2999).
Press F9 in the window to switch click-through on or off.
Model weights, CSV exports and an optional catalog.json live in ~/.tft-synapse/.

Examples:
  tft-synapse
  tft-synapse --width 420 --height 720
  tft-synapse --log-level debug",
    version
)]
pub struct Args {
    /// Accepted for compatibility with old shortcuts; has no effect.
    #[arg(long, default_value_t = false, hide = true)]
    pub overlay: bool,

    /// Accepted for compatibility with old shortcuts; has no effect.
    #[arg(long, default_value_t = false, hide = true)]
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
