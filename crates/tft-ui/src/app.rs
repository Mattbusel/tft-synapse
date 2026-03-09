//! Main egui app: wires UI together, manages the background polling loop.

use std::path::PathBuf;
use std::sync::mpsc;
use egui::Context;
use tft_types::GameState;
use tft_advisor::Advisor;
use crate::panels::{augment_panel, stats_panel, status_bar};
use crate::state::{ConnectionStatus, UiState};
use crate::theme;
use tracing::warn;

pub enum AppMessage {
    GameStateUpdate(GameState),
    Error(String),
    Disconnected,
}

pub struct TftSynapseApp {
    ui_state: UiState,
    advisor: Advisor,
    rx: mpsc::Receiver<AppMessage>,
}

impl TftSynapseApp {
    pub fn new(
        model_path: PathBuf,
        rx: mpsc::Receiver<AppMessage>,
    ) -> Result<Self, tft_types::TftError> {
        let advisor = Advisor::new(model_path)?;
        Ok(Self {
            ui_state: UiState::new(),
            advisor,
            rx,
        })
    }

    fn drain_messages(&mut self) {
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                AppMessage::GameStateUpdate(state) => {
                    self.ui_state.set_connected(ConnectionStatus::Connected);

                    if state.is_augment_phase() {
                        match self.advisor.advise(&state) {
                            Ok(rec) => self.ui_state.recommendation = rec,
                            Err(e) => {
                                warn!("Advisor error: {}", e);
                                self.ui_state.last_error = Some(e.to_string());
                            }
                        }
                    } else {
                        self.ui_state.recommendation = None;
                    }

                    self.ui_state.games_trained = self.advisor.games_trained();
                    self.ui_state.game_state = Some(state);
                }
                AppMessage::Error(e) => {
                    self.ui_state.last_error = Some(e);
                    self.ui_state.set_connected(ConnectionStatus::Polling);
                }
                AppMessage::Disconnected => {
                    self.ui_state.set_connected(ConnectionStatus::Disconnected);
                }
            }
        }
    }
}

impl eframe::App for TftSynapseApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        self.drain_messages();

        ctx.request_repaint_after(std::time::Duration::from_millis(500));

        let style = ctx.style();
        let mut visuals = style.visuals.clone();
        visuals.window_fill = theme::BG_DARK;
        visuals.panel_fill = theme::BG_DARK;
        ctx.set_visuals(visuals);

        egui::TopBottomPanel::top("status_bar").show(ctx, |ui| {
            status_bar::render(
                ui,
                self.ui_state.game_state.as_ref(),
                self.ui_state.connection_status.as_ref(),
            );
        });

        egui::SidePanel::right("stats_panel")
            .min_width(180.0)
            .show(ctx, |ui| {
                stats_panel::render(
                    ui,
                    &self.advisor.metrics,
                    self.ui_state.games_trained,
                    self.ui_state.connection_status.as_ref(),
                );
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            augment_panel::render(ui, self.ui_state.recommendation.as_ref());

            if let Some(ref err) = self.ui_state.last_error {
                ui.add_space(8.0);
                ui.separator();
                ui.label(
                    egui::RichText::new(format!("Error: {}", err))
                        .color(theme::SCORE_LOW)
                        .small()
                );
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_message_error_variant() {
        let msg = AppMessage::Error("test".to_string());
        let is_error = matches!(msg, AppMessage::Error(_));
        assert!(is_error);
    }

    #[test]
    fn test_app_message_disconnected_variant() {
        let _msg = AppMessage::Disconnected;
    }

    #[test]
    fn test_ui_state_default_no_recommendation() {
        let state = UiState::new();
        assert!(!state.has_recommendation());
        assert!(state.game_state.is_none());
    }
}
