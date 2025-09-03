use image::{DynamicImage, Pixel, Rgb, RgbImage};
use std::{
    collections::HashMap,
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use crate::error::AppError;
use crate::pipeline::types::EnrichedFrame;
use tower::Service;

#[derive(Debug, Clone)]
pub struct ColorAnalysis {
    pub dominant_colors: Vec<Rgb<u8>>,
    pub color_distributions: HashMap<String, f32>,
    pub high_contrast_areas: Vec<(u32, u32, u32, u32)>,
    pub likely_text_areas: Vec<(u32, u32, u32, u32)>,
    pub menu_indicators: Vec<String>,
}

impl ColorAnalysis {
    pub fn new() -> Self {
        Self {
            dominant_colors: Vec::new(),
            color_distributions: HashMap::new(),
            high_contrast_areas: Vec::new(),
            likely_text_areas: Vec::new(),
            menu_indicators: Vec::new(),
        }
    }
}

pub struct ColorAnalysisService;

impl ColorAnalysisService {
    pub fn new() -> Self {
        Self
    }

    fn analyze_colors(&self, image: &DynamicImage) -> ColorAnalysis {
        let mut analysis = ColorAnalysis::new();

        let rgb_image = image.to_rgb8();

        self.find_dominant_colors(&rgb_image, &mut analysis);

        self.find_high_contrast_areas(&rgb_image, &mut analysis);

        self.find_likely_text_areas(&rgb_image, &mut analysis);

        self.detect_menu_indicators(&rgb_image, &mut analysis);

        analysis
    }

    fn rgb_to_luma(&self, r: u8, g: u8, b: u8) -> f32 {
        // Rec. 709 luminance
        0.2126 * r as f32 + 0.7152 * g as f32 + 0.0722 * b as f32
    }

    fn quantize_rgb(&self, px: &Rgb<u8>, q: u8) -> (u8, u8, u8) {
        // q must divide 256 evenly (e.g., 16 or 32). 16 → 4-bit per channel.
        let step = 256 / q as usize;
        let q1 = ((px[0] as usize / step) * step).min(255) as u8;
        let q2 = ((px[1] as usize / step) * step).min(255) as u8;
        let q3 = ((px[2] as usize / step) * step).min(255) as u8;
        (q1, q2, q3)
    }

    fn clamp_rect(
        &self,
        x: u32,
        y: u32,
        w: u32,
        h: u32,
        max_w: u32,
        max_h: u32,
    ) -> (u32, u32, u32, u32) {
        let x2 = (x + w).min(max_w);
        let y2 = (y + h).min(max_h);
        (x, y, x2, y2)
    }

    fn find_dominant_colors(&self, image: &RgbImage, analysis: &mut ColorAnalysis) {
        use std::collections::HashMap;
        let mut color_counts: HashMap<(u8, u8, u8), u32> = HashMap::new();

        for y in (0..image.height()).step_by(4) {
            for x in (0..image.width()).step_by(4) {
                let px = image.get_pixel(x, y);
                // quantize to reduce noise / unique bins
                let key = self.quantize_rgb(px, 16); // 16 levels per channel
                *color_counts.entry(key).or_insert(0) += 1;
            }
        }

        let mut sorted: Vec<_> = color_counts.into_iter().collect();
        sorted.sort_by_key(|&(_, c)| std::cmp::Reverse(c));

        analysis.dominant_colors = sorted
            .iter()
            .take(5)
            .map(|&((r, g, b), _)| Rgb([r, g, b]))
            .collect();

        // Optional: fill color_distributions as normalized frequencies
        let total: f32 = sorted.iter().map(|(_, c)| *c as f32).sum();
        analysis.color_distributions = sorted
            .iter()
            .take(32)
            .map(|&((r, g, b), c)| (format!("{r},{g},{b}"), (c as f32 / total)))
            .collect();
    }

    fn find_high_contrast_areas(&self, image: &RgbImage, analysis: &mut ColorAnalysis) {
        let (width, height) = image.dimensions();
        let tile: u32 = 16;

        for y in (0..height).step_by(tile as usize) {
            for x in (0..width).step_by(tile as usize) {
                let contrast = self.calculate_local_contrast(image, x, y, tile);
                if contrast > 50.0 {
                    analysis
                        .high_contrast_areas
                        .push(self.clamp_rect(x, y, tile, tile, width, height));
                }
            }
        }
    }

    fn calculate_local_contrast(&self, image: &RgbImage, x: u32, y: u32, size: u32) -> f32 {
        let w = size.min(image.width() - x);
        let h = size.min(image.height() - y);

        if w == 0 || h == 0 {
            return 0.0;
        }

        // One-pass mean/variance (Welford)
        let mut n = 0f32;
        let mut mean = 0f32;
        let mut m2 = 0f32;

        for dy in 0..h {
            for dx in 0..w {
                let p = image.get_pixel(x + dx, y + dy);
                let v = self.rgb_to_luma(p[0], p[1], p[2]);
                n += 1.0;
                let delta = v - mean;
                mean += delta / n;
                m2 += delta * (v - mean);
            }
        }

        if n < 2.0 {
            0.0
        } else {
            (m2 / (n - 1.0)).sqrt()
        }
    }

    fn find_likely_text_areas(&self, image: &RgbImage, analysis: &mut ColorAnalysis) {
        let (w, h) = image.dimensions();
        let row_step = 8u32;
        let min_run = 30u32;

        for y in (0..h).step_by(row_step as usize) {
            let mut run_start: Option<u32> = None;
            let mut run_len: u32 = 0;

            for x in 1..w {
                let p0 = image.get_pixel(x - 1, y);
                let p1 = image.get_pixel(x, y);
                let g = (self.rgb_to_luma(p1[0], p1[1], p1[2])
                    - self.rgb_to_luma(p0[0], p0[1], p0[2]))
                .abs();

                if g > 25.0 {
                    // edge threshold
                    if run_start.is_none() {
                        run_start = Some(x - 1);
                    }
                    run_len += 1;
                } else if run_len > 0 {
                    if run_len >= min_run {
                        let x0 = run_start.unwrap();
                        analysis
                            .likely_text_areas
                            .push((x0, y, x, (y + row_step).min(h)));
                    }
                    run_start = None;
                    run_len = 0;
                }
            }
        }
    }

    fn detect_menu_indicators(&self, image: &RgbImage, analysis: &mut ColorAnalysis) {
        let (width, height) = image.dimensions();

        for y in (0..height).step_by(8) {
            for x in (0..width).step_by(8) {
                if self.is_cursor_pattern(image, x, y) {
                    analysis.menu_indicators.push("cursor".to_string());
                }
            }
        }

        if self.has_button_patterns(image) {
            analysis.menu_indicators.push("button".to_string());
        }
    }

    fn has_button_patterns(&self, image: &image::RgbImage) -> bool {
        // Look for rectangular button-like patterns
        let (width, height) = image.dimensions();
        let mut button_count = 0;

        for y in (0..height).step_by(16) {
            for x in (0..width).step_by(16) {
                if self.is_button_pattern(image, x, y) {
                    button_count += 1;
                }
            }
        }

        button_count >= 2 // At least 2 buttons to consider it a menu
    }

    fn is_button_pattern(&self, image: &RgbImage, x: u32, y: u32) -> bool {
        let size = 16;
        if x + size > image.width() || y + size > image.height() {
            return false;
        }

        let center = image.get_pixel(x + size / 2, y + size / 2);
        let center_l = self.rgb_to_luma(center[0], center[1], center[2]);

        // sample a set of border pixels
        let mut samples = Vec::with_capacity(4 * (size as usize));
        for dx in 0..size {
            samples.push((x + dx, y)); // top
            samples.push((x + dx, y + size - 1)); // bottom
        }
        for dy in 0..size {
            samples.push((x, y + dy)); // left
            samples.push((x + size - 1, y + dy)); // right
        }

        let mut brighter = 0usize;
        let mut total = 0usize;
        for (sx, sy) in samples {
            let p = image.get_pixel(sx, sy);
            let l = self.rgb_to_luma(p[0], p[1], p[2]);
            if l + 5.0 > center_l {
                brighter += 1;
            } // small margin
            total += 1;
        }

        // Require most border pixels to be brighter (a visible border)
        brighter as f32 / total as f32 > 0.75
    }

    fn is_cursor_pattern(&self, image: &RgbImage, x: u32, y: u32) -> bool {
        if x == 0 || y == 0 || x >= image.width() - 1 || y >= image.height() - 1 {
            return false;
        }
        let c = image.get_pixel(x, y);
        let c_l = self.rgb_to_luma(c[0], c[1], c[2]);

        let neighbors = [
            (x - 1, y - 1),
            (x, y - 1),
            (x + 1, y - 1),
            (x - 1, y),
            (x + 1, y),
            (x - 1, y + 1),
            (x, y + 1),
            (x + 1, y + 1),
        ];

        let sum: f32 = neighbors
            .iter()
            .map(|&(nx, ny)| {
                let p = image.get_pixel(nx, ny);
                self.rgb_to_luma(p[0], p[1], p[2])
            })
            .sum();

        let avg = sum / 8.0;
        c_l > avg + 50.0 // threshold still empirical, but now consistent
    }
}

impl Service<EnrichedFrame> for ColorAnalysisService {
    type Response = EnrichedFrame;
    type Error = AppError;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), AppError>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, mut enriched_frame: EnrichedFrame) -> Self::Future {
        let color_analysis = self.analyze_colors(&enriched_frame.image);

        enriched_frame.color_analysis = Some(color_analysis);

        Box::pin(async move { Ok(enriched_frame) })
    }
}
