use crate::representation::{
    Board, Cell, Command, Game, GameStatus, Mark, MoveRejection, TransitionOutcome,
};

const WINNING_LINES: [[u8; 3]; 8] = [
    [0, 1, 2],
    [3, 4, 5],
    [6, 7, 8],
    [0, 3, 6],
    [1, 4, 7],
    [2, 5, 8],
    [0, 4, 8],
    [2, 4, 6],
];

pub fn transition(state: Game, command: Command) -> TransitionOutcome {
    let GameStatus::InProgress { next } = state.status() else {
        return TransitionOutcome::Rejected {
            state,
            reason: MoveRejection::GameAlreadyTerminal,
        };
    };

    match command {
        Command::PlaceMark { cell } => {
            let Some(board) = state.board().with_mark(cell, next) else {
                return TransitionOutcome::Rejected {
                    state,
                    reason: MoveRejection::CellOccupied,
                };
            };
            TransitionOutcome::Accepted {
                state: Game::from_parts(board, status_after_move(&board, next)),
            }
        }
    }
}

pub fn replay(commands: impl IntoIterator<Item = Command>) -> Trace {
    let mut state = Game::new();
    let mut outcomes = Vec::new();

    for command in commands {
        let outcome = transition(state, command);
        if let TransitionOutcome::Accepted { state: next } = outcome {
            state = next;
        }
        outcomes.push(outcome);
    }

    Trace::new(state, outcomes)
}

pub fn status_after_move(board: &Board, mark: Mark) -> GameStatus {
    if let Some(line) = winning_line(board, mark) {
        GameStatus::Won { winner: mark, line }
    } else if board.is_full() {
        GameStatus::Draw
    } else {
        GameStatus::InProgress { next: mark.other() }
    }
}

pub fn winning_line(board: &Board, mark: Mark) -> Option<[Cell; 3]> {
    for line in WINNING_LINES {
        let cells = [
            Cell::from_index(line[0])?,
            Cell::from_index(line[1])?,
            Cell::from_index(line[2])?,
        ];
        if cells.iter().all(|cell| board.mark_at(*cell) == Some(mark)) {
            return Some(cells);
        }
    }
    None
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Trace {
    final_state: Game,
    outcomes: Vec<TransitionOutcome>,
}

impl Trace {
    pub fn new(final_state: Game, outcomes: Vec<TransitionOutcome>) -> Self {
        Self {
            final_state,
            outcomes,
        }
    }

    pub fn final_state(&self) -> Game {
        self.final_state
    }

    pub fn outcomes(&self) -> &[TransitionOutcome] {
        &self.outcomes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cell(index: u8) -> Cell {
        Cell::from_index(index).unwrap()
    }

    #[test]
    fn accepts_first_move_as_x_and_advances_to_o() {
        let outcome = transition(Game::new(), Command::PlaceMark { cell: cell(0) });

        let TransitionOutcome::Accepted { state } = outcome else {
            panic!("first move should be accepted");
        };
        assert_eq!(state.board().mark_at(cell(0)), Some(Mark::X));
        assert_eq!(state.status(), GameStatus::InProgress { next: Mark::O });
    }

    #[test]
    fn rejects_occupied_cell_without_changing_state() {
        let first = transition(Game::new(), Command::PlaceMark { cell: cell(0) });
        let TransitionOutcome::Accepted { state } = first else {
            panic!("first move should be accepted");
        };

        let second = transition(state, Command::PlaceMark { cell: cell(0) });

        assert_eq!(
            second,
            TransitionOutcome::Rejected {
                state,
                reason: MoveRejection::CellOccupied
            }
        );
    }

    #[test]
    fn detects_top_row_win_for_x() {
        let trace = replay([
            Command::PlaceMark { cell: cell(0) },
            Command::PlaceMark { cell: cell(3) },
            Command::PlaceMark { cell: cell(1) },
            Command::PlaceMark { cell: cell(4) },
            Command::PlaceMark { cell: cell(2) },
        ]);

        assert_eq!(
            trace.final_state().status(),
            GameStatus::Won {
                winner: Mark::X,
                line: [cell(0), cell(1), cell(2)]
            }
        );
    }

    #[test]
    fn drawn_game_rejects_further_moves() {
        let trace = replay([
            Command::PlaceMark { cell: cell(0) },
            Command::PlaceMark { cell: cell(1) },
            Command::PlaceMark { cell: cell(2) },
            Command::PlaceMark { cell: cell(4) },
            Command::PlaceMark { cell: cell(3) },
            Command::PlaceMark { cell: cell(5) },
            Command::PlaceMark { cell: cell(7) },
            Command::PlaceMark { cell: cell(6) },
            Command::PlaceMark { cell: cell(8) },
            Command::PlaceMark { cell: cell(0) },
        ]);

        assert_eq!(trace.outcomes().len(), 10);
        assert_eq!(
            trace.outcomes()[9],
            TransitionOutcome::Rejected {
                state: trace.final_state(),
                reason: MoveRejection::GameAlreadyTerminal
            }
        );
        assert_eq!(trace.final_state().status(), GameStatus::Draw);
    }

    #[test]
    fn invalid_cell_is_unrepresentable() {
        assert_eq!(Cell::new(3, 0), None);
        assert_eq!(Cell::from_index(9), None);
    }
}
