use std::env;
use std::process::ExitCode;

use rand::rngs::StdRng;
use rand::Rng;
use rand::SeedableRng;

use carcassonne_backend::domain::game::{greedy_bot, random_bot, BotFn, Game};

#[derive(Clone, Copy, Debug)]
enum BotKind {
    Greedy,
    Random,
}

impl BotKind {
    fn parse(s: &str) -> Option<Self> {
        match s {
            "greedy" => Some(BotKind::Greedy),
            "random" => Some(BotKind::Random),
            _ => None,
        }
    }

    fn make(self, seed: u64) -> BotFn {
        match self {
            BotKind::Greedy => greedy_bot(),
            BotKind::Random => random_bot(seed),
        }
    }

    fn label(self) -> &'static str {
        match self {
            BotKind::Greedy => "greedy",
            BotKind::Random => "random",
        }
    }
}

struct Args {
    bot_a: BotKind,
    bot_b: BotKind,
    pairs: usize,
    base_seed: u64,
}

fn parse_args() -> Result<Args, String> {
    let mut bot_a = BotKind::Greedy;
    let mut bot_b = BotKind::Random;
    let mut pairs = 50usize;
    let mut base_seed = 0u64;
    let argv: Vec<String> = env::args().collect();
    let mut i = 1;
    while i < argv.len() {
        let key = argv[i].as_str();
        let val = argv.get(i + 1).cloned();
        match key {
            "--bot-a" => {
                bot_a = BotKind::parse(&val.ok_or("missing --bot-a value")?)
                    .ok_or("invalid --bot-a (expected greedy|random)")?;
                i += 2;
            }
            "--bot-b" => {
                bot_b = BotKind::parse(&val.ok_or("missing --bot-b value")?)
                    .ok_or("invalid --bot-b (expected greedy|random)")?;
                i += 2;
            }
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
                eprintln!("usage: eval_paired [--bot-a greedy|random] [--bot-b greedy|random] [--pairs N] [--seed N]");
                std::process::exit(0);
            }
            other => return Err(format!("unknown arg: {other}")),
        }
    }
    Ok(Args { bot_a, bot_b, pairs, base_seed })
}

fn play_one(seed: u64, p0: BotKind, p1: BotKind) -> (u32, u32) {
    let mut game = Game::new(2, seed);
    let mut bots: Vec<BotFn> = vec![p0.make(seed * 2), p1.make(seed * 2 + 1)];
    game.play_full_game(&mut bots);
    let s = game.final_scores();
    (s[0], s[1])
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
    let mut a_wins = 0u32;
    let mut ties = 0u32;
    let mut b_wins = 0u32;
    let mut a_total = 0u64;
    let mut b_total = 0u64;

    for i in 0..args.pairs {
        let seed = args.base_seed.wrapping_add(i as u64);
        // Game 1: A is player 0, B is player 1.
        let (a1, b1) = play_one(seed, args.bot_a, args.bot_b);
        // Game 2: B is player 0, A is player 1.
        let (b2, a2) = play_one(seed, args.bot_b, args.bot_a);
        let a_pair = a1 + a2;
        let b_pair = b1 + b2;
        let diff = a_pair as f64 - b_pair as f64;
        diffs.push(diff);
        a_total += a_pair as u64;
        b_total += b_pair as u64;
        if diff > 0.0 {
            a_wins += 1;
        } else if diff < 0.0 {
            b_wins += 1;
        } else {
            ties += 1;
        }
        if (i + 1) % 10 == 0 {
            eprintln!(
                "[{}/{}] mean diff so far: {:.2}",
                i + 1,
                args.pairs,
                diffs.iter().sum::<f64>() / diffs.len() as f64
            );
        }
    }

    let mean = diffs.iter().sum::<f64>() / diffs.len() as f64;
    let var = diffs.iter().map(|d| (d - mean).powi(2)).sum::<f64>() / (diffs.len() - 1).max(1) as f64;
    let stderr = (var / diffs.len() as f64).sqrt();
    let (lo, hi) = bootstrap_ci(&diffs, 2000, 0.05, args.base_seed ^ 0xDEADBEEF);

    let elapsed = started.elapsed();
    println!();
    println!("=== eval_paired: {} (A) vs {} (B) ===", args.bot_a.label(), args.bot_b.label());
    println!("pairs       : {}", args.pairs);
    println!("seed range  : {}..{}", args.base_seed, args.base_seed + args.pairs as u64);
    println!("elapsed     : {:.1}s", elapsed.as_secs_f64());
    println!();
    println!("A total pts : {}  (avg per pair: {:.2})", a_total, a_total as f64 / args.pairs as f64);
    println!("B total pts : {}  (avg per pair: {:.2})", b_total, b_total as f64 / args.pairs as f64);
    println!();
    println!("pair diffs (A - B):");
    println!("  mean      : {:.2}", mean);
    println!("  stderr    : {:.2}", stderr);
    println!("  95% CI    : [{:.2}, {:.2}]", lo, hi);
    println!();
    println!("pair record :  A wins {}  ties {}  B wins {}", a_wins, ties, b_wins);
    let signif = if lo > 0.0 {
        "A is significantly better (CI strictly above 0)"
    } else if hi < 0.0 {
        "B is significantly better (CI strictly below 0)"
    } else {
        "no significant difference (CI crosses 0)"
    };
    println!("verdict     : {signif}");
    ExitCode::SUCCESS
}
