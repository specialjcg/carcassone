use std::env;
use std::process::ExitCode;

use rand::rngs::StdRng;
use rand::Rng;
use rand::SeedableRng;

use carcassonne_backend::domain::game::{greedy_bot, oracle_bot, BotFn, Game, OracleFn};

struct Args {
    pairs: usize,
    base_seed: u64,
}

fn parse_args() -> Result<Args, String> {
    let mut pairs = 50usize;
    let mut base_seed = 0u64;
    let argv: Vec<String> = env::args().collect();
    let mut i = 1;
    while i < argv.len() {
        let key = argv[i].as_str();
        let val = argv.get(i + 1).cloned();
        match key {
            "--pairs" => {
                pairs = val
                    .ok_or("missing --pairs value")?
                    .parse()
                    .map_err(|_| "--pairs must be a positive integer".to_string())?;
                i += 2;
            }
            "--seed" => {
                base_seed = val
                    .ok_or("missing --seed value")?
                    .parse()
                    .map_err(|_| "--seed must be a u64".to_string())?;
                i += 2;
            }
            "-h" | "--help" => {
                eprintln!("usage: oracle_bound [--pairs N] [--seed N]");
                eprintln!("       always plays oracle (free-choice from bag) vs greedy");
                std::process::exit(0);
            }
            other => return Err(format!("unknown arg: {other}")),
        }
    }
    Ok(Args { pairs, base_seed })
}

/// Run one game where `oracle_player` is the seat (0 or 1) using the oracle bot.
/// Returns (oracle_score, greedy_score).
fn play_one(seed: u64, oracle_player: usize) -> (u32, u32) {
    let mut game = Game::new(2, seed);
    let mut greedy: BotFn = greedy_bot();
    let mut oracle: OracleFn = oracle_bot();
    while !game.is_over() {
        if game.current_player == oracle_player {
            let _ = game.play_one_oracle_turn(&mut oracle);
        } else {
            let _ = game.play_one_turn(&mut greedy);
        }
    }
    game.finish();
    let s = game.final_scores();
    let o = s[oracle_player];
    let g = s[1 - oracle_player];
    (o, g)
}

fn bootstrap_ci(diffs: &[f64], n_resamples: usize, alpha: f64, rng_seed: u64) -> (f64, f64) {
    let mut rng = StdRng::seed_from_u64(rng_seed);
    let n = diffs.len();
    let mut means: Vec<f64> = Vec::with_capacity(n_resamples);
    for _ in 0..n_resamples {
        let mut sum = 0.0;
        for _ in 0..n {
            let i = rng.gen_range(0..n);
            sum += diffs[i];
        }
        means.push(sum / n as f64);
    }
    means.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let lo_idx = ((n_resamples as f64) * alpha / 2.0) as usize;
    let hi_idx = ((n_resamples as f64) * (1.0 - alpha / 2.0)) as usize;
    (means[lo_idx], means[hi_idx.min(n_resamples - 1)])
}

fn main() -> ExitCode {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(2);
        }
    };

    let started = std::time::Instant::now();
    let mut diffs: Vec<f64> = Vec::with_capacity(args.pairs);
    let mut o_wins = 0u32;
    let mut ties = 0u32;
    let mut g_wins = 0u32;
    let mut o_total = 0u64;
    let mut g_total = 0u64;

    for i in 0..args.pairs {
        let seed = args.base_seed.wrapping_add(i as u64);
        // Game 1: oracle is player 0.
        let (o1, g1) = play_one(seed, 0);
        // Game 2: oracle is player 1.
        let (o2, g2) = play_one(seed, 1);
        let o_pair = o1 + o2;
        let g_pair = g1 + g2;
        let diff = o_pair as f64 - g_pair as f64;
        diffs.push(diff);
        o_total += o_pair as u64;
        g_total += g_pair as u64;
        if diff > 0.0 {
            o_wins += 1;
        } else if diff < 0.0 {
            g_wins += 1;
        } else {
            ties += 1;
        }
        eprintln!(
            "[{}/{}] pair diff: {:.0}  (oracle={}, greedy={})  running mean: {:.2}",
            i + 1,
            args.pairs,
            diff,
            o_pair,
            g_pair,
            diffs.iter().sum::<f64>() / diffs.len() as f64
        );
    }

    let mean = diffs.iter().sum::<f64>() / diffs.len() as f64;
    let var =
        diffs.iter().map(|d| (d - mean).powi(2)).sum::<f64>() / (diffs.len() - 1).max(1) as f64;
    let stderr = (var / diffs.len() as f64).sqrt();
    let (lo, hi) = bootstrap_ci(&diffs, 2000, 0.05, args.base_seed ^ 0xC0FFEE);

    let elapsed = started.elapsed();
    println!();
    println!("=== oracle_bound: oracle (free-choice) vs greedy ===");
    println!("pairs       : {}", args.pairs);
    println!("seed range  : {}..{}", args.base_seed, args.base_seed + args.pairs as u64);
    println!("elapsed     : {:.1}s", elapsed.as_secs_f64());
    println!();
    println!("oracle pts  : {}  (avg per pair: {:.2})", o_total, o_total as f64 / args.pairs as f64);
    println!("greedy pts  : {}  (avg per pair: {:.2})", g_total, g_total as f64 / args.pairs as f64);
    println!();
    println!("pair diffs (oracle - greedy):");
    println!("  mean      : {:.2}", mean);
    println!("  stderr    : {:.2}", stderr);
    println!("  95% CI    : [{:.2}, {:.2}]", lo, hi);
    println!();
    println!("pair record :  oracle wins {}  ties {}  greedy wins {}", o_wins, ties, g_wins);
    let verdict = if lo > 0.0 {
        if mean >= 30.0 {
            "ORACLE >> GREEDY (>= 30 pts/pair) — exploitable signal, ML viable"
        } else {
            "oracle > greedy but small margin — ML edge would be marginal"
        }
    } else if hi < 0.0 {
        "greedy > oracle (impossible, suggests bug)"
    } else {
        "no significant difference — Carcassonne is GREEDY-SOLVED, kill ML project"
    };
    println!("verdict     : {verdict}");
    ExitCode::SUCCESS
}
