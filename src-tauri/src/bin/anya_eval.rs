//! CLI entry for the real AgentRunner eval harness.
//!
//! Usage:
//!   cargo run --bin anya-eval -- --tasks ../eval/tasks --results ../eval/results
//!   cargo run --bin anya-eval -- --no-challenge --filter empty

use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

use peek_lib::eval_harness::{run_eval, EvalOptions, LiveEvalConfig};

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let mut options = EvalOptions {
        challenges: true,
        compact: true,
        plan_mode: false,
        tasks_dir: PathBuf::from("eval/tasks"),
        results_dir: PathBuf::from("eval/results"),
        filter: None,
        seeds: 1,
        live: None,
        include_office: false,
    };

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--live" => match LiveEvalConfig::from_env() {
                Ok(config) => options.live = Some(config),
                Err(error) => {
                    eprintln!("{error}");
                    return ExitCode::FAILURE;
                }
            },
            "--include-office" => options.include_office = true,
            "--no-challenge" => options.challenges = false,
            "--no-compact" => options.compact = false,
            "--plan-mode" => options.plan_mode = true,
            "--tasks" => {
                i += 1;
                options.tasks_dir = PathBuf::from(args.get(i).expect("--tasks needs a path"));
            }
            "--results" => {
                i += 1;
                options.results_dir = PathBuf::from(args.get(i).expect("--results needs a path"));
            }
            "--filter" => {
                i += 1;
                options.filter = Some(args.get(i).expect("--filter needs a value").clone());
            }
            "--seeds" => {
                i += 1;
                options.seeds = args
                    .get(i)
                    .expect("--seeds needs a number")
                    .parse()
                    .expect("invalid --seeds");
            }
            "--help" | "-h" => {
                eprintln!(
                    "anya-eval [--live (ANYA_EVAL_ENDPOINT, ANYA_EVAL_MODEL, ANYA_EVAL_API_KEY)] [--tasks DIR] [--results DIR] [--filter ID] [--seeds N] [--no-challenge] [--no-compact] [--plan-mode] [--include-office]"
                );
                return ExitCode::SUCCESS;
            }
            other => {
                eprintln!("unknown arg: {other}");
                return ExitCode::FAILURE;
            }
        }
        i += 1;
    }

    // Resolve relative paths from repo root when launched from src-tauri.
    if !options.tasks_dir.exists() {
        let alt = PathBuf::from("..").join(&options.tasks_dir);
        if alt.exists() {
            options.tasks_dir = alt;
        }
    }
    if options.results_dir.as_os_str() == "eval/results" && !PathBuf::from("eval").exists() {
        options.results_dir = PathBuf::from("..").join("eval/results");
    }

    let report = tauri::async_runtime::block_on(run_eval(options));
    match report {
        Ok(report) => {
            println!(
                "pass_rate={:.1}% passed={} failed={} challenges={} compact={} plan_mode={}",
                report.pass_rate * 100.0,
                report.passed,
                report.failed,
                report.options.challenges,
                report.options.compact,
                report.options.plan_mode
            );
            for result in &report.results {
                if !result.passed {
                    println!("FAIL {} {:?}", result.id, result.errors);
                }
            }
            if report.failed > 0 {
                ExitCode::FAILURE
            } else {
                ExitCode::SUCCESS
            }
        }
        Err(error) => {
            eprintln!("eval failed: {error}");
            ExitCode::FAILURE
        }
    }
}
