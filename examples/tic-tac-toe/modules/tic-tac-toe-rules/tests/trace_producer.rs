use serde_json::json;
use tic_tac_toe_rules::{
    transition_record, Board, Cell, Command, Game, GameStatus, Mark, TicTacToeInput,
    TicTacToeTransitionRecord,
};

fn case_name(value: &impl std::fmt::Debug) -> String {
    format!("{value:?}")
}

fn record_json(record: &TicTacToeTransitionRecord, scenario_start: bool) -> serde_json::Value {
    json!({
        "scenario_start": scenario_start,
        "state_before": case_name(&record.state_before),
        "state_after": case_name(&record.state_after),
        "input": case_name(&record.input),
        "output": {
            "next_state": case_name(&record.output.next_state),
            "events": record.output.events.iter().map(case_name).collect::<Vec<_>>(),
            "commands": record.output.commands.iter().map(case_name).collect::<Vec<_>>(),
            "effects": record.output.effects.iter().map(case_name).collect::<Vec<_>>(),
            "reply": record.output.reply.as_ref().map(case_name),
            "rejection": record.output.rejection.as_ref().map(case_name),
        },
        "source": {
            "file": record.source.file,
            "function": record.source.function,
            "branch": record.source.branch,
        },
    })
}

fn place(index: u8) -> TicTacToeInput {
    TicTacToeInput::Command(Command::PlaceMark {
        cell: Cell::from_index(index).unwrap(),
    })
}

fn in_progress(marks: &[(u8, Mark)], next: Mark) -> Game {
    let board = marks.iter().fold(Board::empty(), |board, (index, mark)| {
        board
            .with_mark(Cell::from_index(*index).unwrap(), *mark)
            .unwrap()
    });
    Game::from_parts(board, GameStatus::InProgress { next })
}

#[test]
fn produce_transition_trace() {
    let Ok(output) = std::env::var("RMS_TRACE_OUTPUT") else {
        return;
    };
    let continued = transition_record(Game::new(), place(0));
    let occupied = transition_record(continued.state_after, place(0));
    let winning = transition_record(
        in_progress(
            &[(0, Mark::X), (1, Mark::X), (3, Mark::O), (4, Mark::O)],
            Mark::X,
        ),
        place(2),
    );
    let after_win = transition_record(winning.state_after, place(5));
    let drawing = transition_record(
        in_progress(
            &[
                (0, Mark::X),
                (1, Mark::O),
                (2, Mark::X),
                (3, Mark::X),
                (4, Mark::O),
                (5, Mark::O),
                (6, Mark::O),
                (7, Mark::X),
            ],
            Mark::X,
        ),
        place(8),
    );
    let after_draw = transition_record(drawing.state_after, place(0));
    let records = [
        (true, continued),
        (false, occupied),
        (true, winning),
        (false, after_win),
        (true, drawing),
        (false, after_draw),
    ];
    let document = json!({
        "spec": "rms/trace-bundle/v0.1",
        "machine": "TicTacToeMachine",
        "records": records
            .iter()
            .map(|(scenario_start, record)| record_json(record, *scenario_start))
            .collect::<Vec<_>>(),
    });

    std::fs::write(output, serde_json::to_vec_pretty(&document).unwrap()).unwrap();
    assert_eq!(records.len(), 6);
}
