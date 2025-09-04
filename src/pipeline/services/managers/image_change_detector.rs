use image::DynamicImage;
use imghash::{ImageHasher, perceptual::PerceptualHasher};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use uuid::Uuid;

/// Manages image change detection using perceptual hashing
/// Extracted from the monolithic AIPipelineService for better separation of concerns
pub struct ImageChangeDetector {
    hasher: Arc<PerceptualHasher>,
    hash_distance_history: HashMap<Uuid, VecDeque<usize>>,
    cached_small_images: HashMap<Uuid, DynamicImage>,
    change_threshold: usize,
    history_window_size: usize,
}

impl ImageChangeDetector {
    pub fn new() -> Self {
        Self {
            hasher: Arc::new(PerceptualHasher::default()),
            hash_distance_history: HashMap::new(),
            cached_small_images: HashMap::new(),
            change_threshold: 5,
            history_window_size: 5,
        }
    }

    pub fn with_threshold(mut self, threshold: usize) -> Self {
        self.change_threshold = threshold;
        self
    }

    pub fn with_history_window(mut self, window_size: usize) -> Self {
        self.history_window_size = window_size;
        self
    }

    /// Detect if image has changed significantly compared to the last frame
    pub fn detect_change(&mut self, client_id: Uuid, current_image: &DynamicImage) -> bool {
        // Downscale current image for faster processing
        let small_current = current_image.resize(64, 64, image::imageops::FilterType::Nearest);

        // Check if we have a previous image to compare against
        if let Some(last_small) = self.cached_small_images.get(&client_id) {
            let last_hash = self.hasher.hash_from_img(last_small);
            let current_hash = self.hasher.hash_from_img(&small_current);
            let distance = last_hash.distance(&current_hash).unwrap_or(0);

            // Update rolling window of distances
            let history = self
                .hash_distance_history
                .entry(client_id)
                .or_insert_with(|| VecDeque::with_capacity(self.history_window_size));

            if history.len() >= self.history_window_size {
                history.pop_front();
            }
            history.push_back(distance);

            // Compute median distance for stability
            let mut sorted: Vec<usize> = history.iter().copied().collect();
            sorted.sort_unstable();
            let median_distance = sorted[sorted.len() / 2];

            // Cache current image for next comparison
            self.cached_small_images.insert(client_id, small_current);

            median_distance > self.change_threshold
        } else {
            // First frame for this client - cache it but don't report change
            self.cached_small_images.insert(client_id, small_current);
            false
        }
    }

    /// Get the current median distance for a client (for debugging)
    pub fn get_median_distance(&self, client_id: &Uuid) -> Option<usize> {
        self.hash_distance_history
            .get(client_id)
            .and_then(|history| {
                if history.is_empty() {
                    None
                } else {
                    let mut sorted: Vec<usize> = history.iter().copied().collect();
                    sorted.sort_unstable();
                    Some(sorted[sorted.len() / 2])
                }
            })
    }

    /// Get recent distance history for a client (for debugging)
    pub fn get_distance_history(&self, client_id: &Uuid) -> Option<Vec<usize>> {
        self.hash_distance_history
            .get(client_id)
            .map(|history| history.iter().copied().collect())
    }

    /// Clear cached data for a client (when client disconnects)
    pub fn clear_client_data(&mut self, client_id: &Uuid) {
        self.hash_distance_history.remove(client_id);
        self.cached_small_images.remove(client_id);
    }

    /// Get current change threshold
    pub fn get_threshold(&self) -> usize {
        self.change_threshold
    }

    /// Update change threshold dynamically
    pub fn set_threshold(&mut self, threshold: usize) {
        self.change_threshold = threshold;
    }

    /// Get statistics about image change detection
    pub fn get_stats(&self) -> ImageChangeStats {
        ImageChangeStats {
            tracked_clients: self.cached_small_images.len(),
            total_history_entries: self.hash_distance_history.values().map(|v| v.len()).sum(),
            current_threshold: self.change_threshold,
            history_window_size: self.history_window_size,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ImageChangeStats {
    pub tracked_clients: usize,
    pub total_history_entries: usize,
    pub current_threshold: usize,
    pub history_window_size: usize,
}

impl Default for ImageChangeDetector {
    fn default() -> Self {
        Self::new()
    }
}
