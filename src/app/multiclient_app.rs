use crate::config::Settings;
use crate::emulator::EmulatorClient;
use crate::error::AppError;
use crate::intake::client::manager::{ClientManager, ClientManagerHandle};
use crate::network::server::Server;
use crate::pipeline::{
    EnrichedFrame, GameAction,
    services::{
        AIPipelineFactory, PerformanceOptimizedPipelineFactory,
        image::analysis::{SceneAnalysisConfig, SceneAnalysisOrchestrator},
        orchestration::UIPipelineAdapter,
    },
};
use tokio::sync::mpsc::error::TryRecvError as MpscTryRecvError;
use tokio::sync::{broadcast, mpsc};
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::app::views::{View, client_view::ClientView};

pub enum UiUpdate {
    ClientList(Vec<Uuid>),
}

pub struct MultiClientApp {
    frame_rx: broadcast::Receiver<EnrichedFrame>,
    show_frame: bool,
    selected_client: Option<Uuid>,
    client_manager: ClientManager,
    client_manager_handle: ClientManagerHandle,
    server_task: JoinHandle<()>,
    ui_update_rx: mpsc::Receiver<UiUpdate>,
    ui_update_tx: mpsc::Sender<UiUpdate>,
    client_id_task: JoinHandle<()>,
    client_ids: Vec<Uuid>,
    cached_frame: Option<EnrichedFrame>,
    ai_pipeline_adapter: UIPipelineAdapter,
    scene_analysis_orchestrator: SceneAnalysisOrchestrator,
    errors: Vec<AppError>,
}

impl MultiClientApp {
    pub fn new(
        frame_rx: broadcast::Receiver<EnrichedFrame>,
        client_manager: ClientManager,
        client_manager_handle: ClientManagerHandle,
        mut server: Server,
        ai_pipeline_adapter: UIPipelineAdapter,
    ) -> Self {
        let (ui_update_tx, ui_update_rx) = mpsc::channel::<UiUpdate>(100);
        let server_task = tokio::spawn(async move {
            server.start().await.expect("Server task died");
        });

        let clone_handle = client_manager_handle.clone();
        let clone_tx = ui_update_tx.clone();

        let client_id_task = tokio::spawn(async move {
            loop {
                let client_ids = clone_handle.list_clients().await;
                debug!("Client IDs to update: {:?}", client_ids);
                match clone_tx.send(UiUpdate::ClientList(client_ids)).await {
                    Ok(_) => {
                        debug!("Client list update sent");
                    }
                    Err(e) => {
                        error!("Error sending client list update: {:?}", e);
                    }
                }
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        });

        Self {
            frame_rx,
            show_frame: true,
            selected_client: None,
            client_manager,
            client_manager_handle,
            server_task,
            ui_update_rx,
            ui_update_tx,
            client_id_task,
            client_ids: Vec::new(),
            cached_frame: None,
            ai_pipeline_adapter,
            scene_analysis_orchestrator: SceneAnalysisOrchestrator::new(
                SceneAnalysisConfig::pokemon_optimized(),
            )
            .expect("Failed to create scene analysis orchestrator"),
            errors: Vec::new(),
        }
    }

    pub fn start_gui(settings: &Settings) {
        let options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size(egui::vec2(1280.0, 720.0))
                .with_title("PokeBot Visualization - Multi Client View"),
            ..Default::default()
        };

        let (frame_tx, frame_rx) = broadcast::channel::<EnrichedFrame>(10000);
        let (action_tx, mut _action_rx) = mpsc::channel::<(Uuid, GameAction)>(1000);

        let (client_manager, client_manager_handle) = ClientManager::new(frame_tx.clone());

        let server = Server::new(3344, client_manager_handle.clone());

        // Create performance-optimized AI pipeline for maximum FPS
        let mut ai_pipeline =
            PerformanceOptimizedPipelineFactory::create_ultra_fast_pipeline(action_tx.clone())
                .expect("Failed to create performance-optimized AI pipeline");
        let ai_pipeline_adapter = ai_pipeline.get_ui_adapter();

        // Spawn a task to route actions from the AI to the correct client
        let client_manager_handle_clone = client_manager_handle.clone();
        tokio::spawn(async move {
            while let Some((client_id, action)) = _action_rx.recv().await {
                client_manager_handle_clone
                    .send_action_to_client(client_id, action)
                    .await;
            }
        });

        // Spawn a task for the AI pipeline to process frames
        let mut ai_frame_rx = frame_tx.subscribe();
        tokio::spawn(async move {
            loop {
                match ai_frame_rx.recv().await {
                    Ok(frame) => {
                        if let Err(e) = ai_pipeline.process_frame(frame).await {
                            error!("AI pipeline failed to process frame: {}", e);
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        warn!("AI pipeline lagged behind, skipping {} frames", n);
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        info!("Frame channel closed, AI pipeline shutting down.");
                        break;
                    }
                }
            }
        });

        let _result = eframe::run_native(
            "PokeBot Visualization - Multi Client View",
            options,
            Box::new(move |_cc| {
                Ok(Box::new(MultiClientApp::new(
                    frame_rx,
                    client_manager,
                    client_manager_handle,
                    server,
                    ai_pipeline_adapter,
                )))
            }),
        );
    }
}

impl eframe::App for MultiClientApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        match self.ui_update_rx.try_recv() {
            Ok(UiUpdate::ClientList(client_ids)) => {
                self.client_ids = client_ids;
            }
            Err(MpscTryRecvError::Empty) => {}
            Err(MpscTryRecvError::Disconnected) => {
                error!("Client list update receiver disconnected");
            }
        };

        // Main UI
        egui::TopBottomPanel::top("Client Selector")
            .resizable(true)
            .show(ctx, |ui| {
                ui.heading("PokeBot Visualization - Multi Client View");
                ui.separator();

                egui::ComboBox::from_label("Active Client.")
                    .selected_text(
                        self.selected_client
                            .map(|id| id.to_string())
                            .unwrap_or("None".to_string()),
                    )
                    .show_ui(ui, |ui| {
                        for client_id in &self.client_ids {
                            let client_name = format!("Client {}", client_id);
                            ui.selectable_value(
                                &mut self.selected_client,
                                Some(*client_id),
                                client_name,
                            );
                        }
                    });

                ui.add_space(4.0);
                ui.separator();
                ui.add_space(4.0);

                // Compact AI status row
                let dbg = self.ai_pipeline_adapter.get_debug_snapshot();
                ui.horizontal_wrapped(|ui| {
                    ui.strong("AI Status:");

                    // Show current Pokemon Black game state
                    if let Some(frame) = &self.cached_frame {
                        if let Some(state) = &frame.state {
                            ui.label(format!("Scene: {:?}", state.scene));
                            ui.label(format!("Location: {:?}", state.location_type));
                            if let Some(location) = &state.current_location {
                                ui.label(format!("Area: {}", location));
                            }
                            if state.in_tall_grass {
                                ui.label("🌱 In Tall Grass!");
                            }
                            ui.label(format!("Pokemon: {}", state.pokemon_count));
                            ui.label(format!("Badges: {}/8", state.badges_earned));
                        } else {
                            ui.label("Scene: No State");
                        }
                    } else {
                        ui.label("Scene: No Frame");
                    }

                    if let Some((mac, ticks)) = dbg.active_macro {
                        ui.label(format!("macro {:?} ({} ticks)", mac, ticks));
                    } else {
                        ui.label("macro -");
                    }
                    if let Some(md) = dbg.median_distance {
                        ui.label(format!("median Δ {}", md));
                    }
                });
            });

        egui::TopBottomPanel::bottom("error_panel")
            .resizable(true)
            .show(ctx, |ui| {
                ui.heading("Error Log");
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for error in self.errors.iter().rev() {
                        ui.label(format!("[ERROR] {}", error));
                    }
                });
            });

        if self.show_frame {
            egui::CentralPanel::default().show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    if let Some(selected_client) = &self.selected_client {
                        match self.frame_rx.try_recv() {
                            Ok(mut frame) => {
                                // Annotate the frame with scene detection for UI display
                                let scene = self
                                    .scene_analysis_orchestrator
                                    .detect_scene_sync(&frame.image);
                                if let Some(state) = &mut frame.state {
                                    state.scene = scene;
                                } else {
                                    frame.state = Some(crate::pipeline::State {
                                        scene,
                                        player_position: (0.0, 0.0),
                                        pokemon_count: 0,
                                        current_location: None,
                                        location_type:
                                            crate::pipeline::types::LocationType::Unknown,
                                        pokemon_party: Vec::new(),
                                        pokedex_seen: 0,
                                        pokedex_caught: 0,
                                        badges_earned: 0,
                                        story_progress:
                                            crate::pipeline::types::StoryProgress::GameStart,
                                        in_tall_grass: false,
                                        menu_cursor_position: None,
                                        battle_turn: None,
                                        last_encounter_steps: 0,
                                        encounter_chain: 0,
                                    });
                                }
                                self.cached_frame = Some(frame);
                            }
                            Err(broadcast::error::TryRecvError::Lagged(n)) => {
                                warn!("UI lagged behind, skipping {} frames", n);
                            }
                            Err(broadcast::error::TryRecvError::Closed) => {
                                let err = AppError::Ui(
                                    "Frame receiver disconnected. This can happen during shutdown."
                                        .to_string(),
                                );
                                self.errors.push(err);
                            }
                            Err(broadcast::error::TryRecvError::Empty) => {}
                        }

                        // Display AI statistics (shared snapshot)
                        ui.heading("AI Pipeline Statistics");
                        let stats = self.ai_pipeline_adapter.get_stats_shared();
                        ui.label(format!(
                            "Frames Processed: {}",
                            stats.total_frames_processed
                        ));
                        ui.label(format!("Decisions Made: {}", stats.total_decisions_made));
                        ui.label(format!(
                            "Average Confidence: {:.2}",
                            stats.average_confidence
                        ));
                        ui.label(format!("Proc FPS: {:.1}", stats.frames_per_sec));
                        ui.label(format!("Decision FPS: {:.1}", stats.decisions_per_sec));
                        ui.label(format!("Actions Sent: {}", stats.total_actions_sent));

                        if let Some(last_time) = stats.last_decision_time {
                            ui.label(format!(
                                "Last Decision: {:?} ago",
                                std::time::Instant::now().duration_since(last_time)
                            ));
                        }

                        ui.separator();

                        // Timing Statistics for Bottleneck Detection
                        ui.heading("Performance Bottlenecks (μs)");
                        let timing = &stats.timing;

                        egui::Grid::new("timing_grid").striped(true).show(ui, |ui| {
                            ui.label("Component");
                            ui.label("EWMA");
                            ui.label("Last");
                            ui.label("Max");
                            ui.end_row();

                            ui.label("Analyze Situation");
                            ui.label(format!("{:.0}", timing.analyze_situation_us));
                            ui.label(format!("{}", timing.last_analyze_situation_us));
                            ui.label(format!("{}", timing.max_analyze_situation_us));
                            ui.end_row();

                            ui.label("Hash Distance");
                            ui.label(format!("{:.0}", timing.hash_distance_us));
                            ui.label(format!("{}", timing.last_hash_distance_us));
                            ui.label(format!("{}", timing.max_hash_distance_us));
                            ui.end_row();

                            ui.label("Policy Inference");
                            ui.label(format!("{:.0}", timing.policy_inference_us));
                            ui.label(format!("{}", timing.last_policy_inference_us));
                            ui.label(format!("{}", timing.max_policy_inference_us));
                            ui.end_row();

                            ui.label("Macro Selection");
                            ui.label(format!("{:.0}", timing.macro_selection_us));
                            ui.label(format!("{}", timing.last_macro_selection_us));
                            ui.label(format!("{}", timing.max_macro_selection_us));
                            ui.end_row();

                            ui.label("Reward Processing");
                            ui.label(format!("{:.0}", timing.reward_processing_us));
                            ui.label(format!("{}", timing.last_reward_processing_us));
                            ui.label(format!("{}", timing.max_reward_processing_us));
                            ui.end_row();

                            ui.label("Experience Collection");
                            ui.label(format!("{:.0}", timing.experience_collection_us));
                            ui.label(format!("{}", timing.last_experience_collection_us));
                            ui.label(format!("{}", timing.max_experience_collection_us));
                            ui.end_row();

                            ui.label("Action Send");
                            ui.label(format!("{:.0}", timing.action_send_us));
                            ui.label(format!("{}", timing.last_action_send_us));
                            ui.label(format!("{}", timing.max_action_send_us));
                            ui.end_row();

                            ui.strong("TOTAL FRAME");
                            ui.strong(format!("{:.0}", timing.total_frame_us));
                            ui.strong(format!("{}", timing.last_total_frame_us));
                            ui.strong(format!("{}", timing.max_total_frame_us));
                            ui.end_row();
                        });

                        ui.separator();

                        // Recent Decisions (compact)
                        ui.heading("Recent Decisions");
                        if let Some(cid) = self.selected_client {
                            let list = self.ai_pipeline_adapter.get_client_decisions(&cid);
                            let shown = list.iter().rev().take(8);
                            egui::Grid::new("recent_decisions_grid")
                                .striped(true)
                                .show(ui, |ui| {
                                    ui.label("Action");
                                    ui.label("Conf");
                                    ui.label("Reason");
                                    ui.end_row();
                                    for d in shown {
                                        ui.label(format!("{:?}", d.action));
                                        ui.label(format!("{:.2}", d.confidence));
                                        ui.label(egui::RichText::new(&d.reasoning).small());
                                        ui.end_row();
                                    }
                                });
                        }

                        if let Some(frame) = &self.cached_frame {
                            ui.heading(format!("Detailed View - Client {}", selected_client));
                            let mut client_view = ClientView::new(*selected_client, frame.clone());
                            client_view.draw(ui);
                        }
                    } else {
                        ui.heading("No client selected");
                    }
                });
            });
        }
        ctx.request_repaint();
    }
}
