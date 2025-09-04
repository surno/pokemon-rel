/// High-performance image change detector with aggressive optimizations
use image::DynamicImage;
use std::collections::HashMap;
use uuid::Uuid;

/// Fast image change detector that uses simple pixel sampling instead of perceptual hashing
pub struct FastImageChangeDetector {
    cached_checksums: HashMap<Uuid, u64>,
    change_threshold: f32,
    sample_rate: u32, // Sample every Nth pixel
}

impl FastImageChangeDetector {
    pub fn new() -> Self {
        Self {
            cached_checksums: HashMap::new(),
            change_threshold: 0.05, // 5% change threshold
            sample_rate: 16,        // Sample every 16th pixel for speed
        }
    }

    pub fn with_threshold(mut self, threshold: f32) -> Self {
        self.change_threshold = threshold.clamp(0.0, 1.0);
        self
    }

    pub fn with_sample_rate(mut self, rate: u32) -> Self {
        self.sample_rate = rate.max(1);
        self
    }

    /// Ultra-fast change detection using simple checksum comparison
    pub fn detect_change_fast(&mut self, client_id: Uuid, current_image: &DynamicImage) -> bool {
        // Compute fast checksum of downscaled image
        let current_checksum = self.compute_fast_checksum(current_image);

        if let Some(&last_checksum) = self.cached_checksums.get(&client_id) {
            // Simple checksum comparison (much faster than perceptual hashing)
            let changed = current_checksum != last_checksum;
            self.cached_checksums.insert(client_id, current_checksum);
            changed
        } else {
            // First frame - cache but don't report change
            self.cached_checksums.insert(client_id, current_checksum);
            false
        }
    }

    /// Compute fast checksum by sampling pixels
    fn compute_fast_checksum(&self, image: &DynamicImage) -> u64 {
        // Downscale to 32x32 for speed (instead of 64x64)
        let small = image.resize(32, 32, image::imageops::FilterType::Nearest);
        let rgb = small.to_rgb8();
        let (width, height) = rgb.dimensions();

        let mut checksum: u64 = 0;
        let mut pixel_count = 0u32;

        // Sample every sample_rate pixels for extreme speed
        for y in (0..height).step_by(self.sample_rate as usize) {
            for x in (0..width).step_by(self.sample_rate as usize) {
                if let Some(pixel) = rgb.get_pixel_checked(x, y) {
                    let [r, g, b] = pixel.0;

                    // Simple hash combining RGB values
                    let pixel_hash = (r as u64) << 16 | (g as u64) << 8 | (b as u64);
                    checksum = checksum.wrapping_mul(31).wrapping_add(pixel_hash);
                    pixel_count += 1;
                }
            }
        }

        // Include pixel count in checksum to detect major structural changes
        checksum.wrapping_mul(31).wrapping_add(pixel_count as u64)
    }

    /// Alternative: Even faster change detection using average brightness
    pub fn detect_change_brightness(
        &mut self,
        client_id: Uuid,
        current_image: &DynamicImage,
    ) -> bool {
        let brightness = self.compute_average_brightness(current_image);

        if let Some(&last_brightness) = self.cached_checksums.get(&client_id) {
            let brightness_diff =
                ((brightness as i64 - last_brightness as i64).abs() as f32) / 255.0;
            let changed = brightness_diff > self.change_threshold;
            self.cached_checksums.insert(client_id, brightness as u64);
            changed
        } else {
            self.cached_checksums.insert(client_id, brightness as u64);
            false
        }
    }

    /// Compute average brightness (extremely fast)
    fn compute_average_brightness(&self, image: &DynamicImage) -> u8 {
        // Ultra-fast: sample just 16x16 pixels
        let small = image.resize(16, 16, image::imageops::FilterType::Nearest);
        let rgb = small.to_rgb8();

        let mut total_brightness = 0u32;
        let mut pixel_count = 0u32;

        // Sample every 2nd pixel for maximum speed
        for y in (0..16).step_by(2) {
            for x in (0..16).step_by(2) {
                if let Some(pixel) = rgb.get_pixel_checked(x, y) {
                    let [r, g, b] = pixel.0;
                    total_brightness += r as u32 + g as u32 + b as u32;
                    pixel_count += 1;
                }
            }
        }

        if pixel_count > 0 {
            (total_brightness / pixel_count / 3) as u8
        } else {
            128 // Default middle brightness
        }
    }

    /// Clear cached data for a client
    pub fn clear_client_data(&mut self, client_id: &Uuid) {
        self.cached_checksums.remove(client_id);
    }

    /// Clear all caches
    pub fn clear_all_caches(&mut self) {
        self.cached_checksums.clear();
    }

    /// Get performance statistics
    pub fn get_performance_stats(&self) -> FastChangeDetectorStats {
        FastChangeDetectorStats {
            tracked_clients: self.cached_checksums.len(),
            sample_rate: self.sample_rate,
            change_threshold: self.change_threshold,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FastChangeDetectorStats {
    pub tracked_clients: usize,
    pub sample_rate: u32,
    pub change_threshold: f32,
}

impl Default for FastImageChangeDetector {
    fn default() -> Self {
        Self::new()
    }
}
