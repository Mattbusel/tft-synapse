//! # tft-advisor
//!
//! Decision engine: reads game state, calls ML policy, produces ranked recommendations.

pub mod advisor;
pub mod board_advisor;
pub mod export;
pub mod metrics;
pub mod reasoning;
pub mod session;
pub mod shop_advisor;

pub use advisor::{Advisor, FullRecommendation, Recommendation};
pub use board_advisor::{BoardAdvisor, BoardRecommendation, TraitStatus};
pub use export::{export_history_csv, export_stats_csv, ExportRow};
pub use metrics::AdvisorMetrics;
pub use session::GameSession;
pub use shop_advisor::{RerollRecommendation, ShopAdvisor, ShopRecommendation};
