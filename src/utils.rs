use std::fs::{File, create_dir_all};
use std::io::{BufWriter, Write};
use std::path::Path;

use crate::dist_table::DistTable;
use crate::instance::Instance;
use crate::lacam::{Solution, SolveOutcome};

pub struct Metrics {
    pub solved: bool,
    pub makespan: usize,
    pub sum_of_costs: usize,
    pub sum_of_loss: usize,
    pub comp_time_ms: f64,
    pub loop_count: usize,
    pub seed: u64,
}

impl Metrics {
    pub fn from_outcome(
        outcome: &SolveOutcome,
        instance: &Instance,
        comp_time_ms: f64,
        loop_count: usize,
        seed: u64,
    ) -> Self {
        match outcome {
            SolveOutcome::Solved(solution) => {
                let makespan = compute_makespan(solution);
                let sum_of_costs = compute_sum_of_costs(solution, instance);
                let sum_of_loss = compute_sum_of_loss(solution, instance);
                Self {
                    solved: true,
                    makespan,
                    sum_of_costs,
                    sum_of_loss,
                    comp_time_ms,
                    loop_count,
                    seed,
                }
            }
            _ => Self {
                solved: false,
                makespan: 0,
                sum_of_costs: 0,
                sum_of_loss: 0,
                comp_time_ms,
                loop_count,
                seed,
            },
        }
    }

    pub fn print_summary(&self, outcome: &SolveOutcome) {
        let status = match outcome {
            SolveOutcome::Solved(_) => "solved",
            SolveOutcome::Timeout => "timeout",
            SolveOutcome::NoSolution => "no-solution",
        };
        println!(
            "status={} makespan={} soc={} sol={} loop_cnt={} comp_time_ms={:.2}",
            status,
            self.makespan,
            self.sum_of_costs,
            self.sum_of_loss,
            self.loop_count,
            self.comp_time_ms,
        );
    }
}

pub fn get_makespan_lower_bound(instance: &Instance, dist_table: &mut DistTable) -> usize {
    let mut c: isize = 0;
    for i in 0..instance.num_agents {
        let d = dist_table.get(i, instance.starts[i]);
        if d > c {
            c = d;
        }
    }
    c as usize
}

pub fn get_sum_of_costs_lower_bound(instance: &Instance, dist_table: &mut DistTable) -> usize {
    let mut c: isize = 0;
    for i in 0..instance.num_agents {
        c += dist_table.get(i, instance.starts[i]);
    }
    c as usize
}

fn compute_makespan(solution: &Solution) -> usize {
    if solution.is_empty() {
        0
    } else {
        solution.len() - 1
    }
}

fn compute_sum_of_costs(solution: &Solution, instance: &Instance) -> usize {
    if solution.is_empty() {
        return 0;
    }
    let makespan = solution.len() - 1;
    let mut soc = 0;
    for i in 0..instance.num_agents {
        let mut arrival = 0;
        for t in 0..=makespan {
            if solution[t][i] != instance.goals[i] {
                arrival = t + 1;
            }
        }
        soc += arrival;
    }
    soc
}

fn compute_sum_of_loss(solution: &Solution, instance: &Instance) -> usize {
    if solution.len() < 2 {
        return 0;
    }
    let mut sol = 0;
    for t in 1..solution.len() {
        for i in 0..instance.num_agents {
            let from = solution[t - 1][i];
            let to = solution[t][i];
            if from != instance.goals[i] || to != instance.goals[i] {
                sol += 1;
            }
        }
    }
    sol
}

pub fn write_solution_to_file(
    output_path: &str,
    instance: &Instance,
    outcome: &SolveOutcome,
    metrics: &Metrics,
    map_file: &str,
) -> crate::error::Result<()> {
    let path = Path::new(output_path);
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            create_dir_all(parent)?;
        }
    }

    let file = File::create(path)?;
    let mut w = BufWriter::new(file);

    writeln!(w, "agents={}", instance.num_agents)?;
    writeln!(w, "map_file={}", map_file)?;
    writeln!(w, "solver=LaCAM")?;
    writeln!(w, "solved={}", if metrics.solved { 1 } else { 0 })?;
    writeln!(w, "soc={}", metrics.sum_of_costs)?;
    writeln!(w, "sum_of_loss={}", metrics.sum_of_loss)?;
    writeln!(w, "makespan={}", metrics.makespan)?;
    writeln!(w, "comp_time={}", metrics.comp_time_ms as u64)?;
    writeln!(w, "loop_cnt={}", metrics.loop_count)?;
    writeln!(w, "seed={}", metrics.seed)?;

    write!(w, "starts=")?;
    write_config(&mut w, &instance.starts, instance)?;
    writeln!(w)?;

    write!(w, "goals=")?;
    write_config(&mut w, &instance.goals, instance)?;
    writeln!(w)?;

    writeln!(w, "solution=")?;
    if let SolveOutcome::Solved(solution) = outcome {
        for (t, config) in solution.iter().enumerate() {
            write!(w, "{}:", t)?;
            write_config(&mut w, config, instance)?;
            writeln!(w)?;
        }
    }

    w.flush()?;
    Ok(())
}

fn write_config<W: Write>(
    w: &mut W,
    config: &[usize],
    instance: &Instance,
) -> std::io::Result<()> {
    for (idx, &vertex_id) in config.iter().enumerate() {
        let v = &instance.graph.vertices[vertex_id];
        if idx > 0 {
            write!(w, ",")?;
        }
        write!(w, "({},{})", v.x, v.y)?;
    }
    Ok(())
}
