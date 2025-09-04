use rand::Rng;
use rand::distr::{Distribution, StandardUniform};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum GameAction {
    A = 0,
    B = 1,
    Up = 2,
    Down = 3,
    Left = 4,
    Right = 5,
    Start = 6,
    Select = 7,
    L = 8,
    R = 9,
    X = 10,
}

impl Distribution<GameAction> for StandardUniform {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> GameAction {
        match rng.random_range(0..=10) {
            0 => GameAction::A,
            1 => GameAction::B,
            2 => GameAction::Up,
            3 => GameAction::Down,
            4 => GameAction::Left,
            5 => GameAction::Right,
            6 => GameAction::Start,
            7 => GameAction::Select,
            8 => GameAction::L,
            9 => GameAction::R,
            _ => GameAction::X,
        }
    }
}
