use crate::{
    error::AppError,
    pipeline::{EnrichedFrame, services::image::SceneAnnotationBuilder},
};
use tower::{Service, ServiceBuilder};

pub fn create_image_pipeline()
-> Result<impl Service<EnrichedFrame, Response = EnrichedFrame, Error = AppError>, AppError> {
    let scene_annotation_service = SceneAnnotationBuilder::new(10, 0.01).build();

    Ok(ServiceBuilder::new().service(scene_annotation_service))
}
