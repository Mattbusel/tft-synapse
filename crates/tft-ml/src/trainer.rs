//! Replay buffer and online gradient update logic.

use crate::model::{Gradients, ShallowNet};
use tft_types::{StateTransition, TftError};

const DEFAULT_CAPACITY: usize = 1000;
const LEARNING_RATE: f32 = 1e-4;

/// Circular replay buffer of StateTransitions.
pub struct ReplayBuffer {
    buffer: Vec<StateTransition>,
    capacity: usize,
    head: usize,
    size: usize,
}

impl ReplayBuffer {
    pub fn new(capacity: usize) -> Self {
        let capacity = capacity.max(1);
        Self {
            buffer: Vec::with_capacity(capacity),
            capacity,
            head: 0,
            size: 0,
        }
    }

    pub fn with_default_capacity() -> Self {
        Self::new(DEFAULT_CAPACITY)
    }

    pub fn push(&mut self, transition: StateTransition) {
        if self.size < self.capacity {
            self.buffer.push(transition);
            self.size += 1;
        } else {
            self.buffer[self.head] = transition;
            self.head = (self.head + 1) % self.capacity;
        }
    }

    pub fn len(&self) -> usize {
        self.size
    }
    pub fn is_empty(&self) -> bool {
        self.size == 0
    }

    /// Sample up to `n` items uniformly (deterministic for reproducibility in tests).
    pub fn sample(&self, n: usize, seed: u64) -> Vec<&StateTransition> {
        if self.is_empty() {
            return vec![];
        }
        let take = n.min(self.size);
        let mut indices: Vec<usize> = (0..self.size).collect();
        // Deterministic shuffle via seed
        let mut rng = seed;
        for i in (1..indices.len()).rev() {
            rng = rng
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let j = (rng as usize) % (i + 1);
            indices.swap(i, j);
        }
        indices[..take].iter().map(|&i| &self.buffer[i]).collect()
    }
}

/// Apply a gradient update to a ShallowNet using SGD.
pub fn apply_sgd_update(net: &mut ShallowNet, grads: &Gradients, lr: f32) {
    fn update_layer(weights: &mut [f32], biases: &mut [f32], dw: &[f32], db: &[f32], lr: f32) {
        for (w, &g) in weights.iter_mut().zip(dw.iter()) {
            *w -= lr * g;
        }
        for (b, &g) in biases.iter_mut().zip(db.iter()) {
            *b -= lr * g;
        }
    }
    update_layer(
        &mut net.layer1.weights,
        &mut net.layer1.biases,
        &grads.d_w1,
        &grads.db1,
        lr,
    );
    update_layer(
        &mut net.layer2.weights,
        &mut net.layer2.biases,
        &grads.d_w2,
        &grads.db2,
        lr,
    );
    update_layer(
        &mut net.layer_out.weights,
        &mut net.layer_out.biases,
        &grads.d_wo,
        &grads.dbo,
        lr,
    );
}

/// Run one mini-batch update on the net using transitions from the replay buffer.
pub fn mini_batch_update(
    net: &mut ShallowNet,
    buffer: &ReplayBuffer,
    batch_size: usize,
    seed: u64,
) -> Result<f32, TftError> {
    let samples = buffer.sample(batch_size, seed);
    if samples.is_empty() {
        return Ok(0.0);
    }

    let mut total_loss = 0.0f32;
    let n = samples.len() as f32;

    // Accumulate gradients
    let mut acc_grads: Option<Gradients> = None;
    for t in &samples {
        let reward = t.placement.map(|p| p.to_reward()).unwrap_or(0.5);
        let grads = net.backward(&t.features, t.augment_chosen as usize, reward)?;
        total_loss += reward;
        match acc_grads.as_mut() {
            None => acc_grads = Some(grads),
            Some(acc) => {
                for (a, g) in acc.d_w1.iter_mut().zip(grads.d_w1.iter()) {
                    *a += g;
                }
                for (a, g) in acc.db1.iter_mut().zip(grads.db1.iter()) {
                    *a += g;
                }
                for (a, g) in acc.d_w2.iter_mut().zip(grads.d_w2.iter()) {
                    *a += g;
                }
                for (a, g) in acc.db2.iter_mut().zip(grads.db2.iter()) {
                    *a += g;
                }
                for (a, g) in acc.d_wo.iter_mut().zip(grads.d_wo.iter()) {
                    *a += g;
                }
                for (a, g) in acc.dbo.iter_mut().zip(grads.dbo.iter()) {
                    *a += g;
                }
            }
        }
    }

    if let Some(mut g) = acc_grads {
        // Average gradients
        for x in g
            .d_w1
            .iter_mut()
            .chain(g.db1.iter_mut())
            .chain(g.d_w2.iter_mut())
            .chain(g.db2.iter_mut())
            .chain(g.d_wo.iter_mut())
            .chain(g.dbo.iter_mut())
        {
            *x /= n;
        }
        apply_sgd_update(net, &g, LEARNING_RATE);
    }

    Ok(total_loss / n)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tft_types::Placement;

    fn make_transition(features: Vec<f32>, chosen: u8, placement: u8) -> StateTransition {
        StateTransition {
            features,
            augment_chosen: chosen,
            placement: Some(Placement(placement)),
        }
    }

    #[test]
    fn test_replay_buffer_push_and_len() {
        let mut buf = ReplayBuffer::new(10);
        assert_eq!(buf.len(), 0);
        buf.push(make_transition(vec![0.0; 4], 0, 1));
        assert_eq!(buf.len(), 1);
    }

    #[test]
    fn test_replay_buffer_capacity_wraps() {
        let mut buf = ReplayBuffer::new(3);
        for i in 0u8..5 {
            buf.push(make_transition(vec![i as f32], 0, 4));
        }
        assert_eq!(buf.len(), 3);
    }

    #[test]
    fn test_replay_buffer_sample_empty() {
        let buf = ReplayBuffer::new(10);
        assert_eq!(buf.sample(5, 0).len(), 0);
    }

    #[test]
    fn test_replay_buffer_sample_respects_n() {
        let mut buf = ReplayBuffer::new(100);
        for i in 0u8..20 {
            buf.push(make_transition(vec![i as f32], 0, 4));
        }
        assert_eq!(buf.sample(5, 42).len(), 5);
    }

    #[test]
    fn test_replay_buffer_sample_cant_exceed_size() {
        let mut buf = ReplayBuffer::new(100);
        buf.push(make_transition(vec![0.5], 0, 3));
        assert_eq!(buf.sample(50, 0).len(), 1);
    }

    #[test]
    fn test_mini_batch_update_empty_buffer_returns_ok() {
        use crate::model::ShallowNet;
        let mut net = ShallowNet::new(4, 8, 4, 3);
        let buf = ReplayBuffer::new(10);
        let result = mini_batch_update(&mut net, &buf, 8, 0);
        assert!(result.is_ok());
    }

    #[test]
    fn test_mini_batch_update_changes_weights() {
        use crate::model::ShallowNet;
        let mut net = ShallowNet::new(4, 8, 4, 3);
        let initial_w = net.layer1.weights.clone();
        let mut buf = ReplayBuffer::new(100);
        for _ in 0..10 {
            buf.push(make_transition(vec![0.5, 0.3, 0.1, 0.9], 1, 2));
        }
        mini_batch_update(&mut net, &buf, 8, 42).expect("update failed in test");
        assert_ne!(
            net.layer1.weights, initial_w,
            "weights should change after update"
        );
    }

    #[test]
    fn test_replay_buffer_is_empty_on_new() {
        let buf = ReplayBuffer::new(10);
        assert!(buf.is_empty());
    }

    #[test]
    fn test_replay_buffer_not_empty_after_push() {
        let mut buf = ReplayBuffer::new(10);
        buf.push(make_transition(vec![0.1], 0, 5));
        assert!(!buf.is_empty());
    }
}
