//! # tft-advisor
//!
//! Decision engine: reads game state, calls ML policy, produces ranked recommendations.

pub mod advisor;
pub mod board_advisor;
pub mod carry_advisor;
pub mod economy_advisor;
pub mod export;
pub mod item_advisor;
pub mod metrics;
pub mod opponent_tracker;
pub mod pool_tracker;
pub mod positioning_advisor;
pub mod reasoning;
pub mod round_timer;
pub mod session;
pub mod shop_advisor;

pub use advisor::{Advisor, FullRecommendation, Recommendation};
pub use board_advisor::{BoardAdvisor, BoardRecommendation, TraitStatus};
pub use carry_advisor::{CarryAdvisor, CarryCandidate};
pub use economy_advisor::{EconomyAction, EconomyAdvice, EconomyAdvisor, StreakType};
pub use export::{export_history_csv, export_stats_csv, ExportRow};
pub use item_advisor::{ItemAdvisor, ItemRecommendation};
pub use metrics::AdvisorMetrics;
pub use opponent_tracker::{LobbyAnalysis, OpponentAnalysis, OpponentTracker, ThreatLevel};
pub use pool_tracker::{PoolEntry, PoolStatus, PoolTracker};
pub use positioning_advisor::{
    BoardLayout, HexPosition, PositionRecommendation, PositionRole, PositioningAdvisor,
};
pub use round_timer::{EventType, RoundTimer, StageAwareness, UpcomingEvent};
pub use session::{AugmentDecision, GameSession, ReviewEntry};
pub use shop_advisor::{RerollRecommendation, ShopAdvisor, ShopRecommendation};
