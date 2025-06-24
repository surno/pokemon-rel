use crate::{
    error::AppError,
    pipeline::{EnrichedFrame, services::image::SceneAnnotationBuilder},
};
use tower::{Service, ServiceBuilder};

pub fn create_image_pipeline()
-> impl Service<EnrichedFrame, Response = EnrichedFrame, Error = AppError> {
    SceneAnnotationBuilder::new(10, 0.01).build()
}
