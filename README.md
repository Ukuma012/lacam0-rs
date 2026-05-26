# lacam0-rs

A Rust port of [Kei18/lacam0](https://github.com/Kei18/lacam0), an anytime
LaCAM-based multi-agent path finding (MAPF) solver.

## Acknowledgement

This crate is a Rust port of [Kei18/lacam0](https://github.com/Kei18/lacam0)
by Keisuke Okumura, distributed under the MIT License. The original
copyright notice is retained in [`LICENCE.txt`](LICENCE.txt).

## Usage

```rust
use lacam0_rs::{Instance, SolverOpts, solve};

let inst = Instance::new("path/to/map.map", starts, goals)?;
let opts = SolverOpts { star: false };
let result = solve(&inst, /* time_limit_sec */ 5.0, /* seed */ 0, &opts);
```

## Visualization

`utils::write_solution_to_file` writes the solution in the text format consumed
directly by [Kei18/mapf-visualizer](https://github.com/Kei18/mapf-visualizer)
(timestep lines of `t:(x,y),(x,y),...`):

```rust
use lacam0_rs::utils::{Metrics, write_solution_to_file};

let metrics = Metrics::from_outcome(&outcome, &inst, comp_time_ms, loop_cnt, seed);
write_solution_to_file("out/solution.txt", &inst, &outcome, &metrics, "path/to/map.map")?;
```

Then:

```sh
mapf-visualizer path/to/map.map out/solution.txt
```

## License

MIT. See [`LICENCE.txt`](LICENCE.txt).
