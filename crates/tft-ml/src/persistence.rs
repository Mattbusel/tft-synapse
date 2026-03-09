//! Save and load neural network weights as JSON.
//! (Using JSON instead of safetensors to avoid extra dependencies.)

use std::path::Path;
use serde::{Deserialize, Serialize};
use tft_types::TftError;
use crate::model::{Linear, ShallowNet};

#[derive(Serialize, Deserialize)]
struct LinearState {
    weights: Vec<f32>,
    biases: Vec<f32>,
    in_size: usize,
    out_size: usize,
}

#[derive(Serialize, Deserialize)]
struct ModelState {
    layer1: LinearState,
    layer2: LinearState,
    layer_out: LinearState,
    input_dim: usize,
    output_dim: usize,
    games_trained: u32,
}

impl From<&Linear> for LinearState {
    fn from(l: &Linear) -> Self {
        Self { weights: l.weights.clone(), biases: l.biases.clone(), in_size: l.in_size, out_size: l.out_size }
    }
}

pub fn save_model(net: &ShallowNet, games_trained: u32, path: &Path) -> Result<(), TftError> {
    let state = ModelState {
        layer1: LinearState::from(&net.layer1),
        layer2: LinearState::from(&net.layer2),
        layer_out: LinearState::from(&net.layer_out),
        input_dim: net.layer1.in_size,
        output_dim: net.layer_out.out_size,
        games_trained,
    };
    let json = serde_json::to_string(&state)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, json)?;
    Ok(())
}

pub fn load_model(path: &Path) -> Result<(ShallowNet, u32), TftError> {
    let json = std::fs::read_to_string(path)?;
    let state: ModelState = serde_json::from_str(&json)?;
    let mut net = ShallowNet::new(state.input_dim, state.layer1.out_size, state.layer2.out_size, state.output_dim);
    net.layer1.weights = state.layer1.weights;
    net.layer1.biases = state.layer1.biases;
    net.layer2.weights = state.layer2.weights;
    net.layer2.biases = state.layer2.biases;
    net.layer_out.weights = state.layer_out.weights;
    net.layer_out.biases = state.layer_out.biases;
    Ok((net, state.games_trained))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env::temp_dir;

    #[test]
    fn test_save_and_load_roundtrip() {
        let net = ShallowNet::new(10, 8, 4, 3);
        let original_w1 = net.layer1.weights.clone();
        let path = temp_dir().join("tft_test_model.json");
        save_model(&net, 42, &path).expect("save failed in test");
        let (loaded, games) = load_model(&path).expect("load failed in test");
        assert_eq!(games, 42);
        assert_eq!(loaded.layer1.weights, original_w1);
        assert_eq!(loaded.layer1.in_size, 10);
        assert_eq!(loaded.layer_out.out_size, 3);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_load_nonexistent_file_errors() {
        let path = Path::new("/nonexistent/path/model.json");
        assert!(load_model(path).is_err());
    }

    #[test]
    fn test_save_creates_directory() {
        let path = temp_dir().join("tft_test_subdir").join("model.json");
        let net = ShallowNet::new(4, 4, 4, 2);
        save_model(&net, 0, &path).expect("save with mkdir failed in test");
        assert!(path.exists());
        let _ = std::fs::remove_dir_all(path.parent().expect("parent failed in test"));
    }
}
