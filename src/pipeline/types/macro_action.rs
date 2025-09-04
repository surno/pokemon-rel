use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MacroAction {
    // Advance dialog/text by pressing A appropriately
    AdvanceDialog,
    // Simple overworld movement primitives
    WalkUp,
    WalkDown,
    WalkLeft,
    WalkRight,
    // Menu interactions
    MenuSelect,
    MenuBack,
    // Skip intros or open main menu
    PressStart,
}
