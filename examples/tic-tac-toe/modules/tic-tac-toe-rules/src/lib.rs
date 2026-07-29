pub mod representation;
pub mod transition;

pub use crate::representation::{
    initial_game, Board, Cell, Command, Game, GameStatus, Mark, MoveRejection, TicTacToeEvent,
    TicTacToeInput, TransitionOutcome,
};
pub use crate::transition::{
    replay, transition, transition_record, winning_line, TicTacToeMachine, TicTacToeTransition,
    TicTacToeTransitionRecord, Trace,
};
