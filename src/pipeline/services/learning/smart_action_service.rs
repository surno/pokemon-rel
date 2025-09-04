use crate::{
    error::AppError,
    pipeline::{EnrichedFrame, GameAction, Scene},
};
use rand::Rng;
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
    pub in_dialog: bool,
    pub cursor_row: Option<u32>,
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
    pub fn is_action_successful(
        &self,
        prev_situation: &GameSituation,
        current_situation: &GameSituation,
    ) -> bool {
        // If dialog state advanced or closed, count as success
        if prev_situation.in_dialog != current_situation.in_dialog {
            return true;
        }

        // Cursor movement within menu counts as success
        if prev_situation.cursor_row.is_some()
            && current_situation.cursor_row.is_some()
            && prev_situation.cursor_row != current_situation.cursor_row
        {
            return true;
        }

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

        // Default to false now; we pair with image-change check upstream
        false
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

    pub fn analyze_situation(&self, frame: &EnrichedFrame) -> GameSituation {
        // Analyze the current game situation based on the frame
        let scene = frame
            .state
            .as_ref()
            .map(|s| s.scene)
            .unwrap_or(Scene::Unknown);

        // Heuristics
        let has_text = self.detect_text_simple(&frame.image);
        let has_menu = self.detect_menu_simple(&frame.image);
        let in_dialog = self.detect_dialog_box_bottom(&frame.image);
        let cursor_row = self.detect_menu_cursor_row(&frame.image);
        let has_buttons = has_menu; // Simple assumption for now

        let dominant_colors = self.get_dominant_colors_simple(&frame.image);
        let urgency_level = self.determine_urgency(scene, has_text, has_menu);

        GameSituation {
            scene,
            has_text,
            has_menu,
            has_buttons,
            in_dialog,
            cursor_row,
            dominant_colors,
            urgency_level,
        }
    }

    fn detect_text_simple(&self, image: &image::DynamicImage) -> bool {
        // Simple text detection: look for areas with high contrast
        let rgb_image = image.to_rgb8();
        let (width, height) = rgb_image.dimensions();

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
        let rgb_image = image.to_rgb8();
        let (width, height) = rgb_image.dimensions();

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

    fn detect_menu_cursor_row(&self, image: &image::DynamicImage) -> Option<u32> {
        // Heuristic: in menus, the highlighted row tends to be a bright/dark band.
        // We compute row brightness and look for the row with strongest local contrast.
        let gray = image.to_luma8();
        let (w, h) = gray.dimensions();
        if h < 32 || w < 64 {
            return None;
        }

        let mut best_row = None;
        let mut best_score: f32 = 0.0;

        for y in (0..h).step_by(2) {
            // average brightness of the row (sample every 4 px)
            let mut sum = 0.0f32;
            let mut count = 0.0f32;
            for x in (0..w).step_by(4) {
                sum += gray.get_pixel(x, y)[0] as f32;
                count += 1.0;
            }
            if count == 0.0 {
                continue;
            }
            let row_avg = sum / count;

            // local contrast score: difference from neighbors
            let prev_avg = if y >= 2 {
                let mut s = 0.0;
                let mut c = 0.0;
                for x in (0..w).step_by(4) {
                    s += gray.get_pixel(x, y - 2)[0] as f32;
                    c += 1.0;
                }
                if c == 0.0 { row_avg } else { s / c }
            } else {
                row_avg
            };
            let next_avg = if y + 2 < h {
                let mut s = 0.0;
                let mut c = 0.0;
                for x in (0..w).step_by(4) {
                    s += gray.get_pixel(x, y + 2)[0] as f32;
                    c += 1.0;
                }
                if c == 0.0 { row_avg } else { s / c }
            } else {
                row_avg
            };

            let score = (row_avg - prev_avg).abs() + (row_avg - next_avg).abs();
            if score > best_score {
                best_score = score;
                best_row = Some(y);
            }
        }

        // Require meaningful score
        if best_score > 40.0 { best_row } else { None }
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

    fn detect_dialog_box_bottom(&self, image: &image::DynamicImage) -> bool {
        // Very simple heuristic: look for a wide high-contrast band near the bottom
        let rgb = image.to_rgb8();
        let (w, h) = rgb.dimensions();
        if h < 32 || w < 64 {
            return false;
        }

        // Scan the bottom 20% of the image in horizontal stripes
        let start_y = (h as f32 * 0.8) as u32;
        let mut strong_rows = 0u32;
        let mut total_rows = 0u32;

        for y in (start_y..h).step_by(2) {
            total_rows += 1;
            // sample columns
            let mut transitions = 0u32;
            let mut last_brightness: Option<f32> = None;
            for x in (0..w).step_by(4) {
                let p = rgb.get_pixel(x, y);
                let b = (p[0] as f32 + p[1] as f32 + p[2] as f32) / 3.0;
                if let Some(lb) = last_brightness {
                    if (b - lb).abs() > 35.0 {
                        transitions += 1;
                    }
                }
                last_brightness = Some(b);
            }
            if transitions > (w / 4) / 6 {
                // row has enough contrast transitions
                strong_rows += 1;
            }
        }

        // If enough strong rows found, likely a dialog box region
        total_rows > 0 && (strong_rows as f32 / total_rows as f32) > 0.3
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

    pub fn make_decision(&mut self, situation: &GameSituation) -> ActionDecision {
        const EPSILON: f32 = 0.1; // 10% chance of exploration
        if rand::random::<f32>() < EPSILON {
            let random_action = rand::random::<GameAction>();
            return ActionDecision {
                action: random_action,
                confidence: 0.1,
                reasoning: "Exploring a random action".to_string(),
                expected_outcome: "Unknown".to_string(),
            };
        }

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
                    && hist_situation.in_dialog == situation.in_dialog
                    && hist_situation.cursor_row == situation.cursor_row
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
                if situation.has_text || situation.in_dialog {
                    GameAction::A // Probably need to advance text/dialog
                } else if situation.has_menu {
                    GameAction::A // Probably need to select menu option
                } else {
                    GameAction::A // Default action
                }
            }
            UrgencyLevel::Low => {
                // When not urgent, can explore
                if situation.has_text || situation.in_dialog {
                    GameAction::A // Read/advance dialog
                } else {
                    GameAction::Up // Move around to explore
                }
            }
        }
    }

    pub fn record_experience(
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::types::State;
    use image::DynamicImage;
    use uuid::Uuid;

    fn create_mock_frame(scene: Scene, has_text: bool, has_menu: bool) -> EnrichedFrame {
        // Create a simple mock image (1x1 pixel)
        let image = DynamicImage::new_rgb8(1, 1);

        EnrichedFrame {
            client: Uuid::new_v4(),
            image,
            timestamp: chrono::Utc::now().timestamp_millis(),
            program: 1,
            id: Uuid::new_v4(),
            state: Some(State {
                scene,
                player_position: (0.0, 0.0),
                pokemon_count: 0,
            }),
            action: None,
            color_analysis: None,
        }
    }

    #[test]
    fn test_smart_action_service_creation() {
        let service = SmartActionService::new();
        assert_eq!(service.action_history.len(), 0);
    }

    #[test]
    fn test_basic_rules_setup() {
        let service = SmartActionService::new();

        // Check that rules were set up for different scenes
        assert!(service.scene_rules.contains_key(&Scene::MainMenu));
        assert!(service.scene_rules.contains_key(&Scene::Intro));
        assert!(service.scene_rules.contains_key(&Scene::Unknown));
    }

    #[test]
    fn test_main_menu_decision() {
        let mut service = SmartActionService::new();

        // Create a frame that looks like main menu with buttons
        let frame = create_mock_frame(Scene::MainMenu, false, true);
        let situation = service.analyze_situation(&frame);

        // The mock frame won't trigger image analysis, so we'll test the scene-based logic
        assert_eq!(situation.scene, Scene::MainMenu);

        // Make decision - since no buttons are detected, it should use the rule for no buttons
        let decision = service.make_decision(&situation);
        // The rule for MainMenu with no buttons should match and return Start
        assert_eq!(decision.action, GameAction::Start);
        assert_eq!(decision.confidence, 0.7);
        assert!(decision.reasoning.contains("Start to begin game"));
    }

    #[test]
    fn test_intro_scene_decision() {
        let mut service = SmartActionService::new();

        // Create a frame that looks like intro
        let frame = create_mock_frame(Scene::Intro, true, false);
        let situation = service.analyze_situation(&frame);

        // The mock frame won't trigger image analysis, so we'll test the scene-based logic
        assert_eq!(situation.scene, Scene::Intro);
        assert_eq!(situation.urgency_level, UrgencyLevel::Low);

        // Make decision - should use scene rules
        let decision = service.make_decision(&situation);
        assert_eq!(decision.action, GameAction::A);
        assert_eq!(decision.confidence, 0.7);
        assert!(decision.reasoning.contains("intro"));
    }

    #[test]
    fn test_learning_from_experience() {
        let mut service = SmartActionService::new();

        // Create initial situation
        let initial_frame = create_mock_frame(Scene::Unknown, false, false);
        let initial_situation = service.analyze_situation(&initial_frame);

        // Record a successful experience
        service.record_experience(initial_situation.clone(), GameAction::Up, true);

        // Check that experience was recorded
        assert_eq!(service.action_history.len(), 1);

        // Get learning stats
        let stats = service.get_learning_stats();
        assert_eq!(stats.total_actions, 1);
        assert_eq!(stats.successful_actions, 1);
        assert_eq!(stats.success_rate, 1.0);
    }

    #[test]
    fn test_feedback_loop() {
        let mut service = SmartActionService::new();

        // Create a sequence of frames
        let frames = vec![
            create_mock_frame(Scene::Intro, true, false), // Frame 1: Intro with text
            create_mock_frame(Scene::MainMenu, false, true), // Frame 2: Main menu
            create_mock_frame(Scene::Unknown, false, false), // Frame 3: Unknown scene
        ];

        // Process frames with feedback
        let decisions = service.demonstrate_learning_loop(frames);

        // Should have made 3 decisions
        assert_eq!(decisions.len(), 3);

        // First decision should be for intro (using rules)
        assert_eq!(decisions[0].action, GameAction::A);
        assert!(decisions[0].reasoning.contains("intro"));

        // Second decision should be for main menu (using rules since no buttons detected)
        assert_eq!(decisions[1].action, GameAction::Start);
        assert!(decisions[1].reasoning.contains("Start to begin game"));

        // Third decision should use heuristics
        assert_eq!(decisions[2].action, GameAction::A);
        assert!(decisions[2].reasoning.contains("unknown"));

        // Check that learning occurred
        let stats = service.get_learning_stats();
        assert_eq!(stats.total_actions, 2); // 2 actions recorded (first frame has no previous action)
        assert!(stats.success_rate > 0.0);
    }

    #[test]
    fn test_urgency_levels() {
        let mut service = SmartActionService::new();

        // Test main menu urgency
        let menu_frame = create_mock_frame(Scene::MainMenu, false, true);
        let menu_situation = service.analyze_situation(&menu_frame);
        assert_eq!(menu_situation.urgency_level, UrgencyLevel::Low); // No menu detected in mock

        // Test intro urgency
        let intro_frame = create_mock_frame(Scene::Intro, false, false);
        let intro_situation = service.analyze_situation(&intro_frame);
        assert_eq!(intro_situation.urgency_level, UrgencyLevel::Low);

        // Test unknown scene with text urgency
        let unknown_frame = create_mock_frame(Scene::Unknown, true, false);
        let unknown_situation = service.analyze_situation(&unknown_frame);
        assert_eq!(unknown_situation.urgency_level, UrgencyLevel::Low); // No text detected in mock
    }
}
