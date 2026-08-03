use serde_json::{json, Value};
use tic_tac_toe_rules::{
    initial_game, transition_record, Board, Cell, Command, Game, GameStatus, Mark, MoveRejection,
    TicTacToeEvent, TicTacToeInput, TicTacToeTransitionRecord, TransitionOutcome,
};

fn mark_name(mark: Mark) -> &'static str {
    match mark {
        Mark::X => "X",
        Mark::O => "O",
    }
}

fn parse_mark(value: &Value) -> Mark {
    match value.as_str() {
        Some("X") => Mark::X,
        Some("O") => Mark::O,
        other => panic!("invalid mark {other:?}"),
    }
}

fn board_json(board: &Board) -> Value {
    Value::Array(
        board
            .marks()
            .iter()
            .map(|mark| mark.map(mark_name).map(Value::from).unwrap_or(Value::Null))
            .collect(),
    )
}

fn parse_board(value: &Value) -> Board {
    let mut board = Board::empty();
    let Some(cells) = value.as_array() else {
        return board;
    };
    for (index, mark) in cells.iter().enumerate() {
        if mark.is_null() {
            continue;
        }
        board = board
            .with_mark(
                Cell::from_index(index as u8).expect("board cell"),
                parse_mark(mark),
            )
            .expect("board contains each cell at most once");
    }
    board
}

fn state_json(state: &Game) -> Value {
    match state {
        Game::InProgress { board, next } => json!({
            "name": "InProgress",
            "data": {"board": board_json(board), "next": mark_name(*next)}
        }),
        Game::Won {
            board,
            winner,
            line,
        } => json!({
            "name": "Won",
            "data": {
                "board": board_json(board),
                "winner": mark_name(*winner),
                "line": line.iter().map(|cell| cell.index()).collect::<Vec<_>>()
            }
        }),
        Game::Draw { board } => json!({
            "name": "Draw",
            "data": {"board": board_json(board)}
        }),
    }
}

fn parse_state(value: &Value) -> Game {
    let name = value["name"].as_str().unwrap_or("InProgress");
    let board = parse_board(&value["data"]["board"]);
    match name {
        "InProgress" => Game::from_parts(
            board,
            GameStatus::InProgress {
                next: parse_mark(&value["data"]["next"]),
            },
        ),
        "Won" => {
            let cells = value["data"]["line"]
                .as_array()
                .expect("winning line")
                .iter()
                .map(|value| {
                    Cell::from_index(value.as_u64().expect("cell index") as u8)
                        .expect("valid cell index")
                })
                .collect::<Vec<_>>();
            Game::from_parts(
                board,
                GameStatus::Won {
                    winner: parse_mark(&value["data"]["winner"]),
                    line: [cells[0], cells[1], cells[2]],
                },
            )
        }
        "Draw" => Game::from_parts(board, GameStatus::Draw),
        other => panic!("unknown probe state {other}"),
    }
}

fn parse_input(value: &Value) -> TicTacToeInput {
    assert_eq!(value["kind"], "command");
    assert_eq!(value["name"], "PlaceMark");
    let data = &value["data"];
    let cell = if let Some(index) = data["cell"].as_u64() {
        Cell::from_index(index as u8)
    } else {
        Cell::new(
            data["row"].as_u64().expect("PlaceMark.data.row") as u8,
            data["column"].as_u64().expect("PlaceMark.data.column") as u8,
        )
    }
    .expect("PlaceMark cell is on the board");
    TicTacToeInput::Command(Command::PlaceMark { cell })
}

fn named(name: &str, data: Value) -> Value {
    json!({"name": name, "data": data})
}

fn record_json(record: &TicTacToeTransitionRecord, input: &Value, scenario_start: bool) -> Value {
    let events = record
        .output
        .events
        .iter()
        .map(|event| {
            named(
                match event {
                    TicTacToeEvent::MarkPlaced => "MarkPlaced",
                    TicTacToeEvent::MoveRejected => "MoveRejected",
                },
                json!({}),
            )
        })
        .collect::<Vec<_>>();
    let reply = record.output.reply.as_ref().map(|reply| match reply {
        TransitionOutcome::Accepted { state } => {
            named("Accepted", json!({"state": state_json(state)}))
        }
        TransitionOutcome::Rejected { state, reason } => named(
            "Rejected",
            json!({"state": state_json(state), "reason": format!("{reason:?}")}),
        ),
    });
    let rejection = record.output.rejection.as_ref().map(|rejection| {
        named(
            match rejection {
                MoveRejection::CellOccupied => "CellOccupied",
                MoveRejection::GameAlreadyTerminal => "GameAlreadyTerminal",
            },
            json!({}),
        )
    });
    json!({
        "scenario_start": scenario_start,
        "state_before": state_json(&record.state_before),
        "state_after": state_json(&record.state_after),
        "input": input,
        "output": {
            "next_state": state_json(&record.output.next_state),
            "events": events,
            "commands": [],
            "effects": [],
            "reply": reply,
            "rejection": rejection
        },
        "source": {
            "file": record.source.file,
            "function": record.source.function,
            "branch": record.source.branch
        }
    })
}

#[test]
fn probe_machine() {
    let (Ok(request_path), Ok(output_path)) = (
        std::env::var("RMS_PROBE_REQUEST"),
        std::env::var("RMS_PROBE_OUTPUT"),
    ) else {
        return;
    };
    let request: Value =
        serde_json::from_slice(&std::fs::read(request_path).expect("probe request")).unwrap();
    let output = if request["operation"] == "describe" {
        json!({
            "spec": "rms/machine-probe-description/v0.1",
            "machine": "TicTacToeMachine",
            "initial_state": state_json(&initial_game()),
            "states": [
                {
                    "name": "InProgress",
                    "data_schema": {"type": "object"},
                    "examples": [state_json(&initial_game())]
                },
                {
                    "name": "Won",
                    "data_schema": {"type": "object"},
                    "examples": [{
                        "name": "Won",
                        "data": {
                            "board": ["X", "X", "X", "O", "O", null, null, null, null],
                            "winner": "X",
                            "line": [0, 1, 2]
                        }
                    }]
                },
                {
                    "name": "Draw",
                    "data_schema": {"type": "object"},
                    "examples": [{
                        "name": "Draw",
                        "data": {
                            "board": ["X", "O", "X", "X", "O", "O", "O", "X", "X"]
                        }
                    }]
                }
            ],
            "inputs": [{
                "kind": "command",
                "name": "PlaceMark",
                "data_schema": {
                    "type": "object",
                    "properties": {
                        "row": {"type": "integer", "minimum": 0, "maximum": 2},
                        "column": {"type": "integer", "minimum": 0, "maximum": 2}
                    },
                    "required": ["row", "column"]
                },
                "example": {
                    "kind": "command",
                    "name": "PlaceMark",
                    "data": {"row": 0, "column": 0}
                }
            }]
        })
    } else if request["operation"] == "evaluate" {
        let results = request["cases"]
            .as_array()
            .expect("probe evaluation cases")
            .iter()
            .map(|case| {
                let input = &case["input"];
                let record = transition_record(parse_state(&case["state"]), parse_input(input));
                json!({
                    "id": case["id"],
                    "record": record_json(&record, input, true)
                })
            })
            .collect::<Vec<_>>();
        json!({
            "spec": "rms/machine-probe-evaluation/v0.2",
            "machine": "TicTacToeMachine",
            "results": results
        })
    } else {
        let mut state = if request["start"] == "initial" {
            initial_game()
        } else {
            parse_state(&request["start"])
        };
        let records = request["steps"]
            .as_array()
            .expect("probe steps")
            .iter()
            .enumerate()
            .map(|(index, step)| {
                let input = &step["input"];
                let record = transition_record(state, parse_input(input));
                state = record.state_after;
                record_json(&record, input, index == 0)
            })
            .collect::<Vec<_>>();
        json!({
            "spec": "rms/trace-bundle/v0.1",
            "machine": "TicTacToeMachine",
            "records": records
        })
    };
    std::fs::write(output_path, serde_json::to_vec_pretty(&output).unwrap()).unwrap();
}
