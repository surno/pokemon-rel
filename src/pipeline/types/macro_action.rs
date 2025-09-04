#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
}
