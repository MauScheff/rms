#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Mark {
    X,
    O,
}

impl Mark {
    pub fn other(self) -> Self {
        match self {
            Mark::X => Mark::O,
            Mark::O => Mark::X,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct Cell {
    index: u8,
}

impl Cell {
    pub fn new(row: u8, column: u8) -> Option<Self> {
        if row < 3 && column < 3 {
            Some(Self {
                index: row * 3 + column,
            })
        } else {
            None
        }
    }

    pub fn from_index(index: u8) -> Option<Self> {
        if index < 9 {
            Some(Self { index })
        } else {
            None
        }
    }

    pub fn index(self) -> usize {
        self.index as usize
    }

    pub fn row(self) -> u8 {
        self.index / 3
    }

    pub fn column(self) -> u8 {
        self.index % 3
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Board {
    cells: [Option<Mark>; 9],
}

impl Board {
    pub fn new() -> Self {
        Self::empty()
    }

    pub fn empty() -> Self {
        Self { cells: [None; 9] }
    }

    pub fn mark_at(&self, cell: Cell) -> Option<Mark> {
        self.cells[cell.index()]
    }

    pub fn with_mark(&self, cell: Cell, mark: Mark) -> Option<Self> {
        if self.mark_at(cell).is_some() {
            return None;
        }
        let mut next = *self;
        next.cells[cell.index()] = Some(mark);
        Some(next)
    }

    pub fn is_full(&self) -> bool {
        self.cells.iter().all(Option::is_some)
    }

    pub fn marks(&self) -> [Option<Mark>; 9] {
        self.cells
    }
}

impl Default for Board {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GameStatus {
    InProgress { next: Mark },
    Won { winner: Mark, line: [Cell; 3] },
    Draw,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Game {
    InProgress { board: Board, next: Mark },
    Won {
        board: Board,
        winner: Mark,
        line: [Cell; 3],
    },
    Draw { board: Board },
}

impl Game {
    pub fn new() -> Self {
        Self::InProgress {
            board: Board::empty(),
            next: Mark::X,
        }
    }

    pub fn board(&self) -> &Board {
        match self {
            Self::InProgress { board, .. }
            | Self::Won { board, .. }
            | Self::Draw { board } => board,
        }
    }

    pub fn status(&self) -> GameStatus {
        match self {
            Self::InProgress { next, .. } => GameStatus::InProgress { next: *next },
            Self::Won { winner, line, .. } => GameStatus::Won {
                winner: *winner,
                line: *line,
            },
            Self::Draw { .. } => GameStatus::Draw,
        }
    }

    pub fn from_parts(board: Board, status: GameStatus) -> Self {
        match status {
            GameStatus::InProgress { next } => Self::InProgress { board, next },
            GameStatus::Won { winner, line } => Self::Won {
                board,
                winner,
                line,
            },
            GameStatus::Draw => Self::Draw { board },
        }
    }
}

impl Default for Game {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Command {
    PlaceMark { cell: Cell },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TicTacToeInput {
    Command(Command),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TicTacToeEvent {
    MarkPlaced,
    MoveRejected,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MoveRejection {
    CellOccupied,
    GameAlreadyTerminal,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransitionOutcome {
    Accepted { state: Game },
    Rejected { state: Game, reason: MoveRejection },
}
