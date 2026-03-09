//! # tft-advisor
//!
//! Decision engine: reads game state, calls ML policy, produces ranked recommendations.

pub mod advisor;
pub mod reasoning;
pub mod session;
pub mod metrics;

pub use advisor::{Advisor, Recommendation};
pub use session::GameSession;
pub use metrics::AdvisorMetrics;
