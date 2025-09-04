use crate::error::AppError;
use crate::pipeline::types::{EnrichedFrame, RLPrediction};
use serde::{Deserialize, Serialize};
use std::f32;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use tower::Service;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PPOPolicy {
    // Unnormalized action preferences (logits) for 12 buttons
    action_logits: Vec<f32>,
}

impl PPOPolicy {
    fn new_default() -> Self {
        Self {
            action_logits: vec![0.0; 12],
        }
    }

    fn to_probabilities(&self) -> Vec<f32> {
        // Numerically stable softmax over logits
        if self.action_logits.is_empty() {
            return vec![];
        }
        let max_logit = self
            .action_logits
            .iter()
            .cloned()
            .fold(f32::NEG_INFINITY, f32::max);
        let mut sum = 0.0f32;
        let mut exps = Vec::with_capacity(self.action_logits.len());
        for &l in &self.action_logits {
            let e = (l - max_logit).exp();
            sum += e;
            exps.push(e);
        }
        if sum <= 0.0 || !sum.is_finite() {
            // fallback to uniform
            let n = self.action_logits.len() as f32;
            return vec![1.0 / n; self.action_logits.len()];
        }
        exps.into_iter().map(|e| e / sum).collect()
    }
}

#[derive(Debug, Clone)]
pub struct RLService {
    policy: PPOPolicy,
}

impl RLService {
    const POLICY_PATH: &'static str = "ppo_policy.json";

    pub fn new() -> Self {
        // Best-effort load from disk
        match std::fs::read(Self::POLICY_PATH) {
            Ok(bytes) => match serde_json::from_slice::<PPOPolicy>(&bytes) {
                Ok(policy) => Self { policy },
                Err(_) => Self {
                    policy: PPOPolicy::new_default(),
                },
            },
            Err(_) => Self {
                policy: PPOPolicy::new_default(),
            },
        }
    }

    pub fn save_now_blocking(&self) {
        if let Ok(bytes) = serde_json::to_vec_pretty(&self.policy) {
            let _ = std::fs::write(Self::POLICY_PATH, bytes);
        }
    }

    // Extremely simple online update: nudge the selected action's logit by a small step
    // This is a placeholder until full PPO training loop is integrated.
    pub fn nudge_action(&mut self, action_index: usize, advantage: f32) {
        if action_index >= self.policy.action_logits.len() {
            return;
        }
        let step_size = 0.01f32;
        let capped_adv = advantage.clamp(-1.0, 1.0);
        self.policy.action_logits[action_index] =
            (self.policy.action_logits[action_index] + step_size * capped_adv).clamp(-5.0, 5.0);
    }
}

impl Service<EnrichedFrame> for RLService {
    type Response = RLPrediction;
    type Error = AppError;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, _request: EnrichedFrame) -> Self::Future {
        // Compute probabilities outside the async block to avoid borrowing self across await
        let probs = self.policy.to_probabilities();
        Box::pin(async move {
            let max_p = probs.iter().cloned().fold(0.0f32, f32::max);
            let prediction = RLPrediction {
                action_probabilities: probs,
                value_estimate: 0.0,
                confidence: max_p,
            };
            Ok(prediction)
        })
    }
}
