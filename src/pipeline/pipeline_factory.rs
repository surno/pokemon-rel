use crate::error::AppError;
use crate::intake::client::Client;

pub trait PipelineFactory {
    fn build(&self) -> Result<Client, AppError>;
}
