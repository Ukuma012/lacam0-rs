pub mod deadline;
pub mod dist_table;
pub mod error;
pub mod graph;
pub mod instance;
pub mod lacam;
pub mod utils;
pub mod pibt;

pub use error::{LacamError, Result};
pub use instance::Instance;

use deadline::Deadline;
use dist_table::DistTable;
use lacam::{LaCAM, SolveOutcome};
use utils::Metrics;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Solved,
    Timeout,
    NoSolution,
}

#[derive(Debug, Clone, Copy)]
pub struct SolverOpts {
    pub star: bool,
    pub parallel_dist_init: bool,
}

impl Default for SolverOpts {
    fn default() -> Self {
        Self {
            star: false,
            parallel_dist_init: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SolveResult {
    pub status: Status,
    pub solution: Option<Vec<Vec<usize>>>,
    pub comp_time_ms: f64,
    pub makespan: Option<usize>,
    pub soc: Option<usize>,
    pub sum_of_loss: Option<usize>,
    pub makespan_lb: usize,
    pub soc_lb: usize,
    pub loop_cnt: usize,
}

pub fn solve(
    instance: &Instance,
    time_limit_sec: f32,
    seed: u64,
    opts: &SolverOpts,
) -> SolveResult {
    // Compute lower bounds first — excluded from comp_time_ms.
    let mut dist_table_for_lb = DistTable::new_lazy(instance);
    let makespan_lb = utils::get_makespan_lower_bound(instance, &mut dist_table_for_lb);
    let soc_lb = utils::get_sum_of_costs_lower_bound(instance, &mut dist_table_for_lb);
    drop(dist_table_for_lb);

    // Start the clock here — only the solve itself is timed.
    let deadline = Deadline::new((time_limit_sec * 1000.0) as f64);
    let dist_table = if opts.parallel_dist_init {
        DistTable::new_parallel(instance)
    } else {
        DistTable::new_lazy(instance)
    };
    let mut solver = LaCAM::new(instance, dist_table, deadline.clone(), seed, opts.star);
    let outcome = solver.solve();
    let comp_time_ms = deadline.elapsed_ms();
    let loop_cnt = solver.loop_cnt;

    let m = Metrics::from_outcome(&outcome, instance, comp_time_ms, loop_cnt, seed);

    let (status, solution, makespan, soc, sum_of_loss) = match outcome {
        SolveOutcome::Solved(sol) => (
            Status::Solved,
            Some(sol),
            Some(m.makespan),
            Some(m.sum_of_costs),
            Some(m.sum_of_loss),
        ),
        SolveOutcome::Timeout => (Status::Timeout, None, None, None, None),
        SolveOutcome::NoSolution => (Status::NoSolution, None, None, None, None),
    };

    SolveResult {
        status,
        solution,
        comp_time_ms,
        makespan,
        soc,
        sum_of_loss,
        makespan_lb,
        soc_lb,
        loop_cnt,
    }
}
