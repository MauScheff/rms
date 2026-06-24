use std::{env, process};

use tic_tac_toe_rules::{
    replay, Cell, Command, Game, GameStatus, Mark, MoveRejection, TransitionOutcome,
};

fn main() {
    let mut commands = Vec::new();
    for argument in env::args().skip(1) {
        let Ok(index) = argument.parse::<u8>() else {
            fail("move index must be an integer");
        };
        let Some(cell) = Cell::from_index(index) else {
            fail("move index must be between 0 and 8");
        };
        commands.push(Command::PlaceMark { cell });
    }

    let trace = replay(commands);
    let outcome = trace.outcomes().last();
    println!(
        "{{\"outcome\":{},\"state\":{}}}",
        outcome_json(outcome),
        game_json(trace.final_state())
    );
}

fn fail(message: &str) -> ! {
    eprintln!("{message}");
    process::exit(2);
}

fn outcome_json(outcome: Option<&TransitionOutcome>) -> String {
    match outcome {
        Some(TransitionOutcome::Accepted { state }) => {
            format!("{{\"tag\":\"Accepted\",\"state\":{}}}", game_json(*state))
        }
        Some(TransitionOutcome::Rejected { state, reason }) => format!(
            "{{\"tag\":\"Rejected\",\"reason\":{},\"state\":{}}}",
            rejection_json(*reason),
            game_json(*state)
        ),
        None => "{\"tag\":\"NoMove\"}".to_string(),
    }
}

fn game_json(game: Game) -> String {
    format!(
        "{{\"board\":{},\"status\":{}}}",
        board_json(game),
        status_json(game.status())
    )
}

fn board_json(game: Game) -> String {
    let cells = game
        .board()
        .marks()
        .into_iter()
        .map(|mark| match mark {
            Some(mark) => format!("\"{}\"", mark_label(mark)),
            None => "null".to_string(),
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("[{cells}]")
}

fn status_json(status: GameStatus) -> String {
    match status {
        GameStatus::InProgress { next } => {
            format!(
                "{{\"tag\":\"InProgress\",\"next\":\"{}\"}}",
                mark_label(next)
            )
        }
        GameStatus::Won { winner, line } => format!(
            "{{\"tag\":\"Won\",\"winner\":\"{}\",\"line\":[{},{},{}]}}",
            mark_label(winner),
            cell_json(line[0]),
            cell_json(line[1]),
            cell_json(line[2])
        ),
        GameStatus::Draw => "{\"tag\":\"Draw\"}".to_string(),
    }
}

fn cell_json(cell: Cell) -> String {
    format!(
        "{{\"row\":{},\"column\":{},\"index\":{}}}",
        cell.row(),
        cell.column(),
        cell.index()
    )
}

fn rejection_json(reason: MoveRejection) -> &'static str {
    match reason {
        MoveRejection::CellOccupied => "\"CellOccupied\"",
        MoveRejection::GameAlreadyTerminal => "\"GameAlreadyTerminal\"",
    }
}

fn mark_label(mark: Mark) -> &'static str {
    match mark {
        Mark::X => "X",
        Mark::O => "O",
    }
}
