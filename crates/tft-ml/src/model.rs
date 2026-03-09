//! Shallow neural network: input -> hidden1(ReLU) -> hidden2(ReLU) -> output(logits)

use tft_types::TftError;

/// Activation function: ReLU
fn relu(x: f32) -> f32 { x.max(0.0) }

/// Apply softmax to a slice in-place.
pub fn softmax(logits: &mut Vec<f32>) {
    let max = logits.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let sum: f32 = logits.iter().map(|&x| (x - max).exp()).sum();
    for x in logits.iter_mut() {
        *x = (*x - max).exp() / sum;
    }
}

/// Linear layer: output = W * input + b
/// W has shape (out_size, in_size) stored row-major.
pub struct Linear {
    pub weights: Vec<f32>, // out_size * in_size
    pub biases: Vec<f32>,  // out_size
    pub in_size: usize,
    pub out_size: usize,
}

impl Linear {
    pub fn new_random(in_size: usize, out_size: usize) -> Self {
        // Xavier initialization
        let scale = (2.0 / (in_size + out_size) as f32).sqrt();
        let weights: Vec<f32> = (0..in_size * out_size)
            .map(|i| {
                // Deterministic pseudo-random via simple LCG for reproducibility
                let x = ((i as u64).wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407)) as f32;
                let norm = (x / u64::MAX as f32) * 2.0 - 1.0;
                norm * scale
            })
            .collect();
        let biases = vec![0.0; out_size];
        Self { weights, biases, in_size, out_size }
    }

    pub fn forward(&self, input: &[f32]) -> Vec<f32> {
        let mut output = self.biases.clone();
        for (i, b) in output.iter_mut().enumerate() {
            let row = &self.weights[i * self.in_size..(i + 1) * self.in_size];
            *b += row.iter().zip(input.iter()).map(|(&w, &x)| w * x).sum::<f32>();
        }
        output
    }
}

/// 2-hidden-layer network.
pub struct ShallowNet {
    pub layer1: Linear,
    pub layer2: Linear,
    pub layer_out: Linear,
}

impl ShallowNet {
    pub fn new(input_dim: usize, hidden1: usize, hidden2: usize, output_dim: usize) -> Self {
        Self {
            layer1: Linear::new_random(input_dim, hidden1),
            layer2: Linear::new_random(hidden1, hidden2),
            layer_out: Linear::new_random(hidden2, output_dim),
        }
    }

    /// Forward pass. Returns logits (pre-softmax).
    pub fn forward(&self, input: &[f32]) -> Result<Vec<f32>, TftError> {
        if input.len() != self.layer1.in_size {
            return Err(TftError::Model(format!(
                "input dim mismatch: expected {}, got {}",
                self.layer1.in_size, input.len()
            )));
        }
        let h1: Vec<f32> = self.layer1.forward(input).into_iter().map(relu).collect();
        let h2: Vec<f32> = self.layer2.forward(&h1).into_iter().map(relu).collect();
        Ok(self.layer_out.forward(&h2))
    }

    /// Backward pass (manual backprop) returning weight gradients for a single sample.
    /// `target_idx` is the augment index chosen, `reward` is [0,1].
    pub fn backward(
        &self,
        input: &[f32],
        target_idx: usize,
        reward: f32,
    ) -> Result<Gradients, TftError> {
        if input.len() != self.layer1.in_size {
            return Err(TftError::Model("input dim mismatch in backward".to_string()));
        }

        // Forward pass, saving activations
        let h1_pre: Vec<f32> = self.layer1.forward(input);
        let h1: Vec<f32> = h1_pre.iter().map(|&x| relu(x)).collect();
        let h2_pre: Vec<f32> = self.layer2.forward(&h1);
        let h2: Vec<f32> = h2_pre.iter().map(|&x| relu(x)).collect();
        let mut logits = self.layer_out.forward(&h2);

        // Softmax -> probabilities
        softmax(&mut logits);

        // dL/d_logit = p_i - 1 if i == target, else p_i (cross-entropy loss)
        let mut d_logits = logits.clone();
        if target_idx < d_logits.len() {
            d_logits[target_idx] -= 1.0;
        }
        // Scale by negative reward: higher reward = stronger gradient toward that choice
        for d in d_logits.iter_mut() {
            *d *= 1.0 - reward; // if reward=1, gradient is zero (perfect choice)
        }

        // Output layer gradients
        let n_out = self.layer_out.out_size;
        let n_h2 = self.layer_out.in_size;
        let mut d_wo = vec![0.0f32; n_out * n_h2];
        let dbo = d_logits.clone();
        for (i, &dl) in d_logits.iter().enumerate() {
            for (j, &h) in h2.iter().enumerate() {
                d_wo[i * n_h2 + j] = dl * h;
            }
        }

        // Gradient through h2
        let n_h1 = self.layer2.in_size;
        let mut d_h2 = vec![0.0f32; n_h2];
        for j in 0..n_h2 {
            for (i, &dl) in d_logits.iter().enumerate() {
                d_h2[j] += dl * self.layer_out.weights[i * n_h2 + j];
            }
            // ReLU backward
            d_h2[j] *= if h2_pre[j] > 0.0 { 1.0 } else { 0.0 };
        }

        // Layer 2 gradients
        let mut d_w2 = vec![0.0f32; n_h2 * n_h1];
        let db2 = d_h2.clone();
        for (i, &dh) in d_h2.iter().enumerate() {
            for (j, &h) in h1.iter().enumerate() {
                d_w2[i * n_h1 + j] = dh * h;
            }
        }

        // Gradient through h1
        let n_in = self.layer1.in_size;
        let mut d_h1 = vec![0.0f32; n_h1];
        for j in 0..n_h1 {
            for (i, &dh) in d_h2.iter().enumerate() {
                d_h1[j] += dh * self.layer2.weights[i * n_h1 + j];
            }
            d_h1[j] *= if h1_pre[j] > 0.0 { 1.0 } else { 0.0 };
        }

        // Layer 1 gradients
        let mut d_w1 = vec![0.0f32; n_h1 * n_in];
        let db1 = d_h1.clone();
        for (i, &dh) in d_h1.iter().enumerate() {
            for (j, &x) in input.iter().enumerate() {
                d_w1[i * n_in + j] = dh * x;
            }
        }

        Ok(Gradients { d_w1, db1, d_w2, db2, d_wo, dbo })
    }
}

pub struct Gradients {
    pub d_w1: Vec<f32>,
    pub db1: Vec<f32>,
    pub d_w2: Vec<f32>,
    pub db2: Vec<f32>,
    pub d_wo: Vec<f32>,
    pub dbo: Vec<f32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_net(input_dim: usize, n_out: usize) -> ShallowNet {
        ShallowNet::new(input_dim, 16, 8, n_out)
    }

    #[test]
    fn test_relu_positive() { assert_eq!(relu(1.5), 1.5); }
    #[test]
    fn test_relu_negative() { assert_eq!(relu(-1.0), 0.0); }
    #[test]
    fn test_relu_zero() { assert_eq!(relu(0.0), 0.0); }

    #[test]
    fn test_softmax_sums_to_one() {
        let mut v = vec![1.0, 2.0, 3.0];
        softmax(&mut v);
        let sum: f32 = v.iter().sum();
        assert!((sum - 1.0).abs() < 1e-5, "softmax sum = {}", sum);
    }

    #[test]
    fn test_softmax_all_positive() {
        let mut v = vec![0.5, -1.0, 2.0];
        softmax(&mut v);
        for &p in &v { assert!(p > 0.0); }
    }

    #[test]
    fn test_forward_output_dimension() {
        let net = make_net(10, 5);
        let input = vec![0.5f32; 10];
        let out = net.forward(&input).expect("forward failed in test");
        assert_eq!(out.len(), 5);
    }

    #[test]
    fn test_forward_wrong_input_dim_errors() {
        let net = make_net(10, 5);
        let input = vec![0.5f32; 7]; // wrong
        let result = net.forward(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_forward_all_zeros_input() {
        let net = make_net(10, 5);
        let input = vec![0.0f32; 10];
        let out = net.forward(&input).expect("forward failed in test");
        assert_eq!(out.len(), 5);
    }

    #[test]
    fn test_backward_gradient_shapes() {
        let n_in = 10;
        let n_out = 5;
        let net = ShallowNet::new(n_in, 16, 8, n_out);
        let input = vec![0.5f32; n_in];
        let grads = net.backward(&input, 2, 0.8).expect("backward failed in test");
        assert_eq!(grads.d_w1.len(), 16 * n_in);
        assert_eq!(grads.db1.len(), 16);
        assert_eq!(grads.d_w2.len(), 8 * 16);
        assert_eq!(grads.db2.len(), 8);
        assert_eq!(grads.d_wo.len(), n_out * 8);
        assert_eq!(grads.dbo.len(), n_out);
    }

    #[test]
    fn test_linear_forward_shape() {
        let layer = Linear::new_random(4, 3);
        let out = layer.forward(&[1.0, 0.0, -1.0, 0.5]);
        assert_eq!(out.len(), 3);
    }
}
