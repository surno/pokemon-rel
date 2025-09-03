use crate::{
    error::AppError,
    pipeline::{EnrichedFrame, GameAction, Scene},
};
use std::{
    collections::{HashMap, VecDeque},
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use tower::Service;

#[derive(Debug, Clone)]
pub struct GameSituation {
    pub scene: Scene,
    pub has_text: bool,
    pub has_menu: bool,
    pub has_buttons: bool,
    pub dominant_colors: Vec<String>,
    pub urgency_level: UrgencyLevel,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UrgencyLevel {
    Low,      // Walking around, exploring
    Medium,   // In a menu, choosing options
    High,     // In battle, need to act quickly
    Critical, // Health low, in danger
}

#[derive(Debug, Clone)]
pub struct ActionDecision {
    pub action: GameAction,
    pub confidence: f32,
    pub reasoning: String,
    pub expected_outcome: String,
}

#[derive(Debug, Clone)]
pub struct LearningStats {
    pub total_actions: usize,
    pub successful_actions: usize,
    pub success_rate: f32,
    pub action_history_size: usize,
}

pub struct ActionRule {
    pub condition: Box<dyn Fn(&GameSituation) -> bool + Send + Sync>,
    pub action: GameAction,
    pub priority: u8,
    pub description: String,
}

pub struct SmartActionService {
    // Simple rules for different game situations
    scene_rules: HashMap<Scene, Vec<ActionRule>>,
    // Learning from past experiences
    action_history: VecDeque<(GameSituation, GameAction, bool)>, // situation, action, was_successful
}

impl SmartActionService {
    pub fn new() -> Self {
        let mut service = Self {
            scene_rules: HashMap::new(),
            action_history: VecDeque::new(),
        };

        // Set up basic rules for different game situations
        service.setup_basic_rules();
        service
    }

    // Add feedback method to record action results
    pub fn record_action_result(
        &mut self,
        situation: GameSituation,
        action: GameAction,
        was_successful: bool,
        next_situation: Option<GameSituation>,
    ) {
        // Clone values before moving them
        let situation_clone = situation.clone();
        let action_clone = action.clone();

        // Record the experience for learning
        self.record_experience(situation, action, was_successful);

        // Log the result for debugging
        tracing::info!(
            "Action result recorded: {:?} -> {:?} (success: {})",
            situation_clone.scene,
            action_clone,
            was_successful
        );

        // If we have a next situation, we could analyze the transition
        if let Some(next) = next_situation {
            tracing::debug!(
                "Scene transition: {:?} -> {:?}",
                situation_clone.scene,
                next.scene
            );
        }
    }

    // Method to get learning statistics
    pub fn get_learning_stats(&self) -> LearningStats {
        let total_actions = self.action_history.len();
        let successful_actions = self
            .action_history
            .iter()
            .filter(|(_, _, was_successful)| *was_successful)
            .count();

        let success_rate = if total_actions > 0 {
            successful_actions as f32 / total_actions as f32
        } else {
            0.0
        };

        LearningStats {
            total_actions,
            successful_actions,
            success_rate,
            action_history_size: self.action_history.len(),
        }
    }

    // Method to integrate with pipeline and provide feedback
    pub fn process_frame_with_feedback(
        &mut self,
        current_frame: EnrichedFrame,
        previous_action: Option<GameAction>,
        previous_situation: Option<GameSituation>,
    ) -> (ActionDecision, Option<GameAction>) {
        // Analyze current situation
        let current_situation = self.analyze_situation(&current_frame);

        // If we have a previous action and situation, record the result
        if let (Some(prev_action), Some(prev_situation)) = (previous_action, previous_situation) {
            // Determine if the action was successful based on scene changes
            let was_successful = self.is_action_successful(&prev_situation, &current_situation);

            // Record the feedback
            self.record_action_result(
                prev_situation,
                prev_action,
                was_successful,
                Some(current_situation.clone()),
            );
        }

        // Make decision for current situation
        let decision = self.make_decision(&current_situation);

        // Clone the action for the return tuple
        let action_for_return = decision.action.clone();

        // Return both the decision and the previous action for next iteration
        (decision, Some(action_for_return))
    }

    // Helper method to determine if an action was successful
    fn is_action_successful(
        &self,
        prev_situation: &GameSituation,
        current_situation: &GameSituation,
    ) -> bool {
        // Simple heuristic: if scene changed, action was probably successful
        if prev_situation.scene != current_situation.scene {
            return true;
        }

        // If we're still in the same scene but text/menu state changed, action might be successful
        if prev_situation.has_text != current_situation.has_text {
            return true;
        }

        if prev_situation.has_menu != current_situation.has_menu {
            return true;
        }

        // Default to true for now (we can improve this later)
        true
    }

    // Public method to demonstrate usage in main application
    pub fn demonstrate_learning_loop(&mut self, frames: Vec<EnrichedFrame>) -> Vec<ActionDecision> {
        let mut decisions = Vec::new();
        let mut previous_action: Option<GameAction> = None;
        let mut previous_situation: Option<GameSituation> = None;

        for frame in frames {
            let frame_clone = frame.clone();
            let (decision, next_previous_action) =
                self.process_frame_with_feedback(frame, previous_action, previous_situation);

            decisions.push(decision);
            previous_action = next_previous_action;
            previous_situation = Some(self.analyze_situation(&frame_clone));
        }

        decisions
    }

    fn setup_basic_rules(&mut self) {
        // Rules for Main Menu
        let mut main_menu_rules = Vec::new();
        main_menu_rules.push(ActionRule {
            condition: Box::new(|situation| {
                situation.scene == Scene::MainMenu && situation.has_buttons
            }),
            action: GameAction::A,
            priority: 1,
            description: "Press A to select menu option".to_string(),
        });

        main_menu_rules.push(ActionRule {
            condition: Box::new(|situation| {
                situation.scene == Scene::MainMenu && !situation.has_buttons
            }),
            action: GameAction::Start,
            priority: 2,
            description: "Press Start to begin game".to_string(),
        });

        self.scene_rules.insert(Scene::MainMenu, main_menu_rules);

        // Rules for Intro/Unknown scenes
        let mut intro_rules = Vec::new();
        intro_rules.push(ActionRule {
            condition: Box::new(|situation| {
                situation.scene == Scene::Intro || situation.scene == Scene::Unknown
            }),
            action: GameAction::A,
            priority: 1,
            description: "Press A to advance through intro".to_string(),
        });

        self.scene_rules.insert(Scene::Intro, intro_rules);

        // Create separate rules for Unknown scene
        let unknown_rules = vec![ActionRule {
            condition: Box::new(|situation| situation.scene == Scene::Unknown),
            action: GameAction::A,
            priority: 1,
            description: "Press A to advance through unknown".to_string(),
        }];
        self.scene_rules.insert(Scene::Unknown, unknown_rules);
    }

    fn analyze_situation(&self, frame: &EnrichedFrame) -> GameSituation {
        // Analyze the current game situation based on the frame
        let scene = frame
            .state
            .as_ref()
            .map(|s| s.scene)
            .unwrap_or(Scene::Unknown);

        // For now, we'll use simple heuristics
        // Later we can integrate with the color analysis service
        let has_text = self.detect_text_simple(&frame.image);
        let has_menu = self.detect_menu_simple(&frame.image);
        let has_buttons = has_menu; // Simple assumption for now

        let dominant_colors = self.get_dominant_colors_simple(&frame.image);
        let urgency_level = self.determine_urgency(scene, has_text, has_menu);

        GameSituation {
            scene,
            has_text,
            has_menu,
            has_buttons,
            dominant_colors,
            urgency_level,
        }
    }

    fn detect_text_simple(&self, image: &image::DynamicImage) -> bool {
        // Simple text detection: look for areas with high contrast
        // This is a placeholder - we'll improve this with the color analysis service
        let rgb_image = image.to_rgb8();
        let (width, height) = rgb_image.dimensions();

        // Sample some pixels and check for high contrast
        let mut high_contrast_count = 0;
        let mut total_samples = 0;

        for y in (0..height).step_by(8) {
            for x in (0..width).step_by(8) {
                if x > 0 && y > 0 && x < width - 1 && y < height - 1 {
                    let current = rgb_image.get_pixel(x, y);
                    let left = rgb_image.get_pixel(x - 1, y);
                    let above = rgb_image.get_pixel(x, y - 1);

                    let current_brightness =
                        (current[0] as f32 + current[1] as f32 + current[2] as f32) / 3.0;
                    let left_brightness = (left[0] as f32 + left[1] as f32 + left[2] as f32) / 3.0;
                    let above_brightness =
                        (above[0] as f32 + above[1] as f32 + above[2] as f32) / 3.0;

                    if (current_brightness - left_brightness).abs() > 50.0
                        || (current_brightness - above_brightness).abs() > 50.0
                    {
                        high_contrast_count += 1;
                    }
                    total_samples += 1;
                }
            }
        }

        if total_samples == 0 {
            return false;
        }

        // If more than 20% of samples have high contrast, likely has text
        high_contrast_count as f32 / total_samples as f32 > 0.2
    }

    fn detect_menu_simple(&self, image: &image::DynamicImage) -> bool {
        // Simple menu detection: look for rectangular patterns
        // This is a placeholder - we'll improve this with the color analysis service
        let rgb_image = image.to_rgb8();
        let (width, height) = rgb_image.dimensions();

        // Look for areas with consistent borders
        let mut menu_indicators = 0;

        for y in (0..height).step_by(16) {
            for x in (0..width).step_by(16) {
                if self.looks_like_menu_item(&rgb_image, x, y) {
                    menu_indicators += 1;
                }
            }
        }

        menu_indicators >= 2 // At least 2 menu-like items
    }

    fn looks_like_menu_item(&self, image: &image::RgbImage, x: u32, y: u32) -> bool {
        let size = 16;
        if x + size > image.width() || y + size > image.height() {
            return false;
        }

        // Precompute center brightness once
        let center = image.get_pixel(x + size / 2, y + size / 2);
        let center_brightness = (center[0] as f32 + center[1] as f32 + center[2] as f32) / 3.0;

        // Count border pixels that differ sufficiently from the center
        let mut border_pixels = 0u32;
        let mut high_contrast_border = 0u32;

        for dy in 0..size {
            for dx in 0..size {
                let is_border = dx == 0 || dx == size - 1 || dy == 0 || dy == size - 1;
                if !is_border {
                    continue;
                }

                let p = image.get_pixel(x + dx, y + dy);
                let pb = (p[0] as f32 + p[1] as f32 + p[2] as f32) / 3.0;
                border_pixels += 1;
                if (center_brightness - pb).abs() >= 30.0 {
                    high_contrast_border += 1;
                }
            }
        }

        // Require a strong majority of border pixels to contrast with the center (reduces false positives)
        border_pixels > 0 && (high_contrast_border as f32 / border_pixels as f32) >= 0.7
    }

    fn get_dominant_colors_simple(&self, image: &image::DynamicImage) -> Vec<String> {
        // Simple color analysis - we'll improve this with the color analysis service
        let rgb_image = image.to_rgb8();
        let (width, height) = rgb_image.dimensions();

        let mut color_counts: HashMap<String, u32> = HashMap::new();

        // Sample pixels and categorize colors
        for y in (0..height).step_by(8) {
            for x in (0..width).step_by(8) {
                let pixel = rgb_image.get_pixel(x, y);
                let color_name = self.categorize_color(pixel);
                *color_counts.entry(color_name).or_insert(0) += 1;
            }
        }

        // Return top 3 most common colors
        let mut sorted_colors: Vec<_> = color_counts.iter().collect();
        sorted_colors.sort_by(|a, b| b.1.cmp(a.1));

        sorted_colors
            .iter()
            .take(3)
            .map(|(color, _)| color.to_string())
            .collect()
    }

    fn categorize_color(&self, pixel: &image::Rgb<u8>) -> String {
        let r = pixel[0] as f32;
        let g = pixel[1] as f32;
        let b = pixel[2] as f32;

        // Simple color categorization
        if r > 200.0 && g > 200.0 && b > 200.0 {
            "white".to_string()
        } else if r < 50.0 && g < 50.0 && b < 50.0 {
            "black".to_string()
        } else if r > g + 50.0 && r > b + 50.0 {
            "red".to_string()
        } else if g > r + 50.0 && g > b + 50.0 {
            "green".to_string()
        } else if b > r + 50.0 && b > g + 50.0 {
            "blue".to_string()
        } else if r > 150.0 && g > 150.0 && b < 100.0 {
            "yellow".to_string()
        } else {
            "other".to_string()
        }
    }

    fn determine_urgency(&self, scene: Scene, has_text: bool, has_menu: bool) -> UrgencyLevel {
        match scene {
            Scene::MainMenu => {
                if has_menu {
                    UrgencyLevel::Medium // Need to make a choice
                } else {
                    UrgencyLevel::Low // Just waiting
                }
            }
            Scene::Intro => UrgencyLevel::Low, // Can take time
            Scene::Unknown => {
                if has_text {
                    UrgencyLevel::Medium // Might need to respond
                } else {
                    UrgencyLevel::Low // Just exploring
                }
            }
        }
    }

    fn make_decision(&mut self, situation: &GameSituation) -> ActionDecision {
        // First, try to apply learned rules from experience
        if let Some(learned_action) = self.get_learned_action(situation) {
            return learned_action;
        }

        // Then, apply basic scene rules
        if let Some(rules) = self.scene_rules.get(&situation.scene) {
            // Sort rules by priority without cloning
            let mut ordered: Vec<_> = rules.iter().collect();
            ordered.sort_by_key(|r| r.priority);
            for rule in ordered {
                if (rule.condition)(situation) {
                    return ActionDecision {
                        action: rule.action.clone(),
                        confidence: 0.7,
                        reasoning: rule.description.clone(),
                        expected_outcome: "Follow basic game logic".to_string(),
                    };
                }
            }
        }

        // Fallback: use heuristics based on situation
        let action = self.heuristic_decision(situation);
        ActionDecision {
            action,
            confidence: 0.3, // Low confidence for heuristics
            reasoning: "Using heuristic fallback".to_string(),
            expected_outcome: "Unknown outcome".to_string(),
        }
    }

    fn get_learned_action(&self, situation: &GameSituation) -> Option<ActionDecision> {
        // Look for similar situations in our history
        let similar_experiences: Vec<_> = self
            .action_history
            .iter()
            .filter(|(hist_situation, _, was_successful)| {
                // Simple similarity check - we can improve this
                hist_situation.scene == situation.scene
                    && hist_situation.has_text == situation.has_text
                    && hist_situation.has_menu == situation.has_menu
                    && *was_successful // Only use successful actions
            })
            .collect();

        if similar_experiences.is_empty() {
            return None;
        }

        // Find the most common successful action for this situation
        let mut action_counts: HashMap<GameAction, u32> = HashMap::new();
        for (_, action, _) in similar_experiences {
            *action_counts.entry(action.clone()).or_insert(0) += 1;
        }

        let best_action = action_counts
            .iter()
            .max_by_key(|(_, count)| *count)
            .map(|(action, _)| action.clone())?;

        Some(ActionDecision {
            action: best_action,
            confidence: 0.8, // High confidence for learned actions
            reasoning: "Based on successful past experience".to_string(),
            expected_outcome: "Should work based on history".to_string(),
        })
    }

    fn heuristic_decision(&self, situation: &GameSituation) -> GameAction {
        // Simple heuristics when we don't have specific rules
        match situation.urgency_level {
            UrgencyLevel::Critical => GameAction::A, // Act quickly
            UrgencyLevel::High => GameAction::A,     // Act quickly
            UrgencyLevel::Medium => {
                if situation.has_text {
                    GameAction::A // Probably need to advance text
                } else if situation.has_menu {
                    GameAction::A // Probably need to select menu option
                } else {
                    GameAction::A // Default action
                }
            }
            UrgencyLevel::Low => {
                // When not urgent, can explore
                if situation.has_text {
                    GameAction::A // Read text
                } else {
                    GameAction::Up // Move around to explore
                }
            }
        }
    }

    fn record_experience(
        &mut self,
        situation: GameSituation,
        action: GameAction,
        was_successful: bool,
    ) {
        self.action_history
            .push_back((situation, action, was_successful));

        // Keep history manageable
        if self.action_history.len() > 1000 {
            let _ = self.action_history.pop_front();
        }
    }
}

impl Service<EnrichedFrame> for SmartActionService {
    type Response = ActionDecision;
    type Error = AppError;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), AppError>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, request: EnrichedFrame) -> Self::Future {
        let situation = self.analyze_situation(&request);
        let decision = self.make_decision(&situation);

        Box::pin(async move { Ok(decision) })
    }
}
