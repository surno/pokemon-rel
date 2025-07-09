#[derive(Clone)]
pub enum Input {
    A,
    B,
    Up,
    Down,
    Left,
    Right,
    Start,
    Select,
    L,
    R,
    X,
}

#[derive(Clone)]
pub struct GameAction {
    pub action: Input,
}
