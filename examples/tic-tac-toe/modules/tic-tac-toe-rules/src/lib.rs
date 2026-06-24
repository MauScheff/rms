pub mod representation;
pub mod transition;

pub use crate::representation::{
    Board, Cell, Command, Game, GameStatus, Mark, MoveRejection, TransitionOutcome,
};
pub use crate::transition::{replay, transition, winning_line, Trace};
