use crate::pipeline::types::RawFrame;
use tower::Service;

#[derive(Debug, Clone)]
impl Service<RawFrame> for PreprocessingService {
    type Response = EnrichedFrame;
    type Error = PreprocessingError;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, request: RawFrame) -> Self::Future {
        Box::pin(async move {
            let game_state = GameState::new(0.0, 0.0, 0);
            let features = vec![];

            Ok(EnrichedFrame {
                raw_frame: request,
                game_state,
                features,
            })
        })
    }
}
