use crate::{
    error::AppError,
    pipeline::{EnrichedFrame, services::image::SceneAnnotationServiceBuilder},
};
use tower::{Service, ServiceBuilder};

pub fn create_image_pipeline()
-> impl Service<EnrichedFrame, Response = EnrichedFrame, Error = AppError> {
    SceneAnnotationServiceBuilder::new(10, 0.01).build()
}
