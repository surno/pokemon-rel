use super::frame_context::FrameContext;
use crate::error::AppError;
use async_trait::async_trait;
use std::collections::{HashMap, VecDeque};

/// Chain of Responsibility pattern for processing pipeline
#[async_trait]
pub trait ProcessingStep: Send + Sync {
    async fn process(&mut self, context: &mut FrameContext) -> Result<(), AppError>;
    fn name(&self) -> &'static str;
    
    /// Optional: Return dependencies (step names that must run before this step)
    /// Returns None if no dependencies, or Some with step names
    fn dependencies(&self) -> Option<&[&'static str]> {
        None
    }
    
    /// Optional: Whether this step can run in parallel with other independent steps
    fn can_parallelize(&self) -> bool {
        false
    }
}

/// Step registry entry with metadata
struct StepEntry {
    step: Box<dyn ProcessingStep>,
    name: &'static str,
    index: usize,
}

/// A pipeline that processes frames through a chain of steps
/// Uses industry-standard data structures:
/// - HashMap for O(1) step lookups by name
/// - Vec for maintaining insertion order and fast iteration
/// - VecDeque for efficient step queue management
pub struct ProcessingPipeline {
    /// Steps in execution order (preserves insertion order)
    steps: Vec<StepEntry>,
    /// Fast lookup by step name for dependency resolution
    step_index: HashMap<&'static str, usize>,
    /// Queue of steps ready to execute (for future parallel execution support)
    ready_queue: VecDeque<usize>,
}

impl ProcessingPipeline {
    pub fn new() -> Self {
        Self {
            steps: Vec::new(),
            step_index: HashMap::new(),
            ready_queue: VecDeque::new(),
        }
    }

    /// Add a step to the pipeline
    /// Steps are executed in the order they are added
    pub fn add_step(mut self, step: Box<dyn ProcessingStep>) -> Self {
        let name = step.name();
        let index = self.steps.len();
        
        // Build step index for fast lookups
        self.step_index.insert(name, index);
        
        // Add to steps vector (preserves order)
        self.steps.push(StepEntry {
            step,
            name,
            index,
        });
        
        self
    }

    /// Get a step by name (useful for dependency resolution)
    pub fn get_step_index(&self, name: &str) -> Option<usize> {
        self.step_index.get(name).copied()
    }

    /// Get total number of steps
    pub fn step_count(&self) -> usize {
        self.steps.len()
    }

    /// Process a frame through the pipeline
    /// Currently executes steps sequentially, but structure supports future parallelization
    pub async fn process(&mut self, mut context: FrameContext) -> Result<FrameContext, AppError> {
        // Validate dependencies exist before execution
        self.validate_dependencies()?;
        
        // Execute steps in order (sequential for now, parallel execution can be added later)
        for entry in &mut self.steps {
            tracing::debug!("Processing step: {} (index: {})", entry.name, entry.index);
            
            // Record step start in context metadata
            context.mark_step_start(entry.name);
            
            match entry.step.process(&mut context).await {
                Ok(()) => {
                    context.mark_step_complete(entry.name);
                    tracing::debug้อ("Step {} completed successfully", entry.name);
                }
                Err(e) => {
                    context.mark_step_error(entry.name, &e);
                    tracing::error!("Step {} failed: {}", entry.name, e);
                    return Err(e);
                }
            }
        }
        
        Ok(context)
    }
    
    /// Validate that all step dependencies exist in the pipeline
    fn validate_dependencies(&self) -> Result<(), AppError> {
        for entry in &self.steps {
            if let Some(deps) = entry.step.dependencies() {
                for dep_name in deps {
                    if !self.step_index.contains_key(dep_name) {
                        return Err(AppError::Client(format!(
                            "Step '{}' depends on '{}' which is not in the pipeline",
                            entry.name, dep_name
                        )));
                    }
                }
            }
        }
        Ok(())
    }
    
    /// Get all step names in execution order
    pub fn step_names(&self) -> Vec<&'static str> {
        self.steps.iter().map(|e| e.name).collect()
    }
}