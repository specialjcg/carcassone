use std::env;
use std::process::ExitCode;

use carcassonne_backend::domain::game::{greedy_bot, BotFn, Game};
use carcassonne_backend::domain::snapshot::GameView;

struct Args {
    seed: u64,
    turns: Option<usize>,
    pretty: bool,
}

fn parse_args() -> Result<Args, String> {
    let mut seed = 1u64;
    let mut turns: Option<usize> = Some(8);
    let mut pretty = false;
    let argv: Vec<String> = env::args().collect();
    let mut i = 1;
    while i < argv.len() {
        let key = argv[i].as_str();
        let val = argv.get(i + 1).cloned();
        match key {
            "--seed" => {
                seed = val.ok_or("missing --seed value")?.parse().map_err(|_| "--seed must be a u64".to_string())?;
                i += 2;
            }
            "--turns" => {
                turns = Some(val.ok_or("missing --turns value")?.parse().map_err(|_| "--turns must be a usize".to_string())?);
                i += 2;
            }
            "--full" => {
                turns = None;
                i += 1;
            }
            "--pretty" => {
                pretty = true;
                i += 1;
            }
            "-h" | "--help" => {
                eprintln!("usage: dump_game [--seed N] [--turns N | --full] [--pretty]");
                eprintln!("  Plays a greedy-vs-greedy game (or N turns) and prints GameView JSON.");
                std::process::exit(0);
            }
            other => return Err(format!("unknown arg: {other}")),
        }
    }
    Ok(Args { seed, turns, pretty })
}

fn main() -> ExitCode {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(2);
        }
    };

    let mut game = Game::new(2, args.seed);
    let mut bots: Vec<BotFn> = vec![greedy_bot(), greedy_bot()];
    match args.turns {
        Some(n) => {
            for _ in 0..n {
                if game.is_over() {
                    break;
                }
                let cp = game.current_player;
                let _ = game.play_one_turn(&mut bots[cp]);
            }
        }
        None => game.play_full_game(&mut bots),
    }

    let view = GameView::from_game(&game);
    let json = if args.pretty {
        serde_json::to_string_pretty(&view).expect("serialize")
    } else {
        serde_json::to_string(&view).expect("serialize")
    };
    println!("{json}");
    ExitCode::SUCCESS
}
