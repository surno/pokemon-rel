use crate::{
    error::AppError,
    pipeline::{EnrichedFrame, Scene},
};
use bloomfilter::Bloom;
use image::DynamicImage;
use imghash::{ImageHasher, perceptual::PerceptualHasher};
use std::{
    collections::HashMap,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};
use tower::Service;

#[derive(Debug, Clone)]
pub struct SceneAnnotationBuilder {
    bloom_filters: HashMap<Scene, Bloom<String>>,
    capacity: usize,
    fp_rate: f64,
}

impl SceneAnnotationBuilder {
    pub fn new(capacity: usize, fp_rate: f64) -> Self {
        Self {
            bloom_filters: HashMap::new(),
            capacity,
            fp_rate,
        }
    }

    pub fn with_scene(mut self, scene: Scene, hashes: Vec<String>) -> Self {
        let mut bloom_filter = Bloom::new_for_fp_rate(self.capacity, self.fp_rate).unwrap();
        for hash in hashes {
            bloom_filter.set(&hash);
        }
        self.bloom_filters.insert(scene, bloom_filter);
        self
    }

    pub fn build(self) -> SceneAnnotationService {
        SceneAnnotationService {
            bloom_filters: Arc::new(self.bloom_filters),
        }
    }
}

#[derive(Clone)]
pub struct SceneAnnotationService {
    bloom_filters: Arc<HashMap<Scene, Bloom<String>>>,
}

impl SceneAnnotationService {
    pub fn new(bloom_filters: Arc<HashMap<Scene, Bloom<String>>>) -> Self {
        Self { bloom_filters }
    }

    fn detect_scene(&self, frame: &DynamicImage) -> Scene {
        let hash = self.hash_frame(frame);
        self.bloom_filters
            .iter()
            .find(|(_, filter)| filter.check(&hash))
            .map(|(scene, _)| *scene)
            .unwrap_or(Scene::Unknown)
    }

    fn hash_frame(&self, frame: &DynamicImage) -> String {
        let hash = PerceptualHasher::default().hash_from_img(frame);
        hash.encode()
    }
}

impl Service<EnrichedFrame> for SceneAnnotationService {
    type Response = EnrichedFrame;
    type Error = AppError;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, enriched_frame: EnrichedFrame) -> Self::Future {
        let _ = self.detect_scene(&enriched_frame.image);
        Box::pin(async move {
            todo!()
            // Ok(EnrichedFrame {
            //     raw: enriched_frame.raw,
            //     state: enriched_frame.state,
            //     ml_prediction: enriched_frame.ml_prediction,
            //     game_action: enriched_frame.game_action,
            // })
        })
    }
}
