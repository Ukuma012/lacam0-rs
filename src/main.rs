use clap::Parser;

use lacam0_rs::{
    Instance, SolveResult, SolverOpts, Status, solve,
    lacam::{Solution, SolveOutcome},
    utils::{Metrics, write_solution_to_file},
};

#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long)]
    map: String,

    #[arg(short = 'i', long)]
    scen: Option<String>,

    #[arg(short = 'N', long)]
    num: usize,

    #[arg(short, long, default_value_t = 0)]
    seed: u64,

    #[arg(short, long, default_value_t = 1.0)]
    time_limit_sec: f32,

    #[arg(short, long, default_value = "./build/result.txt")]
    output: String,

    #[arg(long, action = clap::ArgAction::SetTrue)]
    star: bool,

    #[arg(long, action = clap::ArgAction::SetTrue)]
    no_dist_table_init: bool,
}


fn main() -> lacam0_rs::Result<()> {
    let args = Args::parse();

    let instance = match &args.scen {
        Some(scen) => Instance::from_scen(scen, &args.map, args.num)?,
        None => Instance::from_random(&args.map, args.num, args.seed)?,
    };

    let opts = SolverOpts {
        star: args.star,
        parallel_dist_init: !args.no_dist_table_init,
        compute_lb: true,
    };
    let result = solve(&instance, args.time_limit_sec, args.seed, &opts);

    print_summary(&result);

    let (outcome, metrics) = to_outcome_and_metrics(&result, &instance, args.seed);
    write_solution_to_file(&args.output, &instance, &outcome, &metrics, &args.map)?;

    Ok(())
}

fn print_summary(r: &SolveResult) {
    let status = match r.status {
        Status::Solved => "solved",
        Status::Timeout => "timeout",
        Status::NoSolution => "no-solution",
    };
    println!(
        "status={} makespan={} soc={} sol={} loop_cnt={} comp_time_ms={:.2}",
        status,
        r.makespan.unwrap_or(0),
        r.soc.unwrap_or(0),
        r.sum_of_loss.unwrap_or(0),
        r.loop_cnt,
        r.comp_time_ms,
    );
}

fn to_outcome_and_metrics(
    r: &SolveResult,
    instance: &Instance,
    seed: u64,
) -> (SolveOutcome, Metrics) {
    let outcome = match (r.status, &r.solution) {
        (Status::Solved, Some(sol)) => SolveOutcome::Solved(sol.clone() as Solution),
        (Status::Timeout, _) => SolveOutcome::Timeout,
        (Status::NoSolution, _) => SolveOutcome::NoSolution,
        (Status::Solved, None) => SolveOutcome::NoSolution,
    };
    let metrics = Metrics::from_outcome(&outcome, instance, r.comp_time_ms, r.loop_cnt, seed);
    (outcome, metrics)
}
