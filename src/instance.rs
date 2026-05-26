use std::{
    fs::File,
    io::{BufRead, BufReader},
};

use rand::{SeedableRng, rngs::StdRng, seq::SliceRandom};
use regex::Regex;

use crate::graph::{Config, Graph};

#[derive(Debug, Clone)]
pub struct Instance {
    pub graph: Graph,
    pub starts: Config,
    pub goals: Config,
    pub num_agents: usize,
}
impl Instance {
    pub fn new(
        map_filename: &str,
        starts: Vec<usize>,
        goals: Vec<usize>,
    ) -> crate::error::Result<Self> {
        let num_agents = starts.len();
        Ok(Self {
            graph: Graph::from_file(map_filename)?,
            starts,
            goals,
            num_agents,
        })
    }

    pub fn from_random(
        map_filename: &str,
        num_agents: usize,
        seed: u64,
    ) -> crate::error::Result<Self> {
        let graph = Graph::from_file(map_filename)?;
        let k = graph.vertices.len();

        let mut rng = StdRng::seed_from_u64(seed);
        let mut s_indexes: Vec<usize> = (0..k).collect();
        s_indexes.shuffle(&mut rng);
        let mut g_indexes: Vec<usize> = (0..k).collect();
        g_indexes.shuffle(&mut rng);

        let starts: Vec<usize> = s_indexes.into_iter().take(num_agents).collect();
        let goals: Vec<usize> = g_indexes.into_iter().take(num_agents).collect();

        Ok(Self {
            graph,
            starts,
            goals,
            num_agents,
        })
    }

    pub fn from_scen(
        scen_filename: &str,
        map_filename: &str,
        num_agents: usize,
    ) -> crate::error::Result<Self> {
        let graph = Graph::from_file(map_filename)?;
        let mut starts = Vec::new();
        let mut goals = Vec::new();

        let file = File::open(scen_filename)?;
        let reader = BufReader::new(file);

        let re = Regex::new(r"\d+\t.+\.map\t\d+\t\d+\t(\d+)\t(\d+)\t(\d+)\t(\d+)\t.+").unwrap();

        for line in reader.lines() {
            let line = line?;
            let line = line.trim_end_matches('\r');

            if let Some(captures) = re.captures(line) {
                let x_s: usize = captures[1].parse().unwrap();
                let y_s: usize = captures[2].parse().unwrap();
                let x_g: usize = captures[3].parse().unwrap();
                let y_g: usize = captures[4].parse().unwrap();

                if x_s >= graph.width
                    || x_g >= graph.width
                    || y_s >= graph.height
                    || y_g >= graph.height
                {
                    continue;
                }

                let start_index = graph.width * y_s + x_s;
                let goal_index = graph.width * y_g + x_g;

                if start_index < graph.grid.len() && goal_index < graph.grid.len() {
                    let start_node = graph.grid[start_index];
                    let goal_node = graph.grid[goal_index];

                    if let (Some(start_node), Some(goal_node)) = (start_node, goal_node) {
                        starts.push(start_node);
                        goals.push(goal_node);
                    }
                }
            }

            if starts.len() == num_agents {
                break;
            }
        }

        Ok(Self {
            graph,
            starts,
            goals,
            num_agents,
        })
    }

    pub fn from_arrays(
        grid: Vec<Vec<bool>>,
        starts: Vec<(usize, usize)>,
        goals: Vec<(usize, usize)>,
    ) -> crate::error::Result<Self> {
        use crate::error::LacamError;

        if starts.len() != goals.len() {
            return Err(LacamError::Parse {
                field: format!(
                    "starts/goals length mismatch: {} vs {}",
                    starts.len(),
                    goals.len()
                ),
            });
        }

        let graph = Graph::from_array(grid)?;
        let num_agents = starts.len();

        let starts_ids = Self::coords_to_vertex_ids(&graph, &starts, "start")?;
        let goals_ids = Self::coords_to_vertex_ids(&graph, &goals, "goal")?;

        Ok(Self {
            graph,
            starts: starts_ids,
            goals: goals_ids,
            num_agents,
        })
    }

    fn coords_to_vertex_ids(
        graph: &Graph,
        coords: &[(usize, usize)],
        label: &str,
    ) -> crate::error::Result<Vec<usize>> {
        use crate::error::LacamError;

        let mut ids = Vec::with_capacity(coords.len());
        for (i, &(y, x)) in coords.iter().enumerate() {
            if y >= graph.height || x >= graph.width {
                return Err(LacamError::Parse {
                    field: format!("{} #{} out of bounds: (y={}, x={})", label, i, y, x),
                });
            }
            let index = graph.width * y + x;
            match graph.grid[index] {
                Some(vertex_id) => ids.push(vertex_id),
                None => {
                    return Err(LacamError::Parse {
                        field: format!("{} #{} is on obstacle: (y={}, x={})", label, i, y, x),
                    });
                }
            }
        }
        Ok(ids)
    }
}
