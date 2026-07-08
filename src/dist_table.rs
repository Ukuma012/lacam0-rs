use std::collections::VecDeque;

use rayon::prelude::*;

use crate::{graph::Graph, instance::Instance};

#[derive(Debug, Clone)]
pub struct DistTable<'a> {
    pub num_vertices: usize,
    pub table: Vec<Vec<i32>>,
    pub open_queues: Vec<VecDeque<usize>>,
    graph: &'a Graph,
}

impl<'a> DistTable<'a> {
    pub fn new_lazy(instance: &'a Instance) -> Self {
        let max_dist = instance.graph.vertices.len() as i32;
        let mut table = vec![vec![max_dist; instance.graph.vertices.len()]; instance.num_agents];
        let mut open_queues = vec![VecDeque::new(); instance.num_agents];
        for i in 0..instance.num_agents {
            let goal_id = instance.goals[i];
            open_queues[i].push_back(goal_id);
            table[i][goal_id] = 0;
        }

        Self {
            num_vertices: instance.graph.vertices.len(),
            table,
            open_queues,
            graph: &instance.graph,
        }
    }

    pub fn new_parallel(instance: &'a Instance) -> Self {
        let num_vertices = instance.graph.vertices.len();
        let max_dist = num_vertices as i32;
        let mut table = vec![vec![max_dist; num_vertices]; instance.num_agents];

        table
            .par_iter_mut()
            .zip(instance.goals.par_iter())
            .for_each(|(row, &goal_id)| {
                row[goal_id] = 0;
                let mut queue = VecDeque::new();
                queue.push_back(goal_id);
                while let Some(current_vertex_id) = queue.pop_front() {
                    let current_dist = row[current_vertex_id];
                    for &neighbor_id in instance.graph.get_neighbors(current_vertex_id) {
                        if current_dist + 1 < row[neighbor_id] {
                            row[neighbor_id] = current_dist + 1;
                            queue.push_back(neighbor_id);
                        }
                    }
                }
            });

        Self {
            num_vertices,
            table,
            open_queues: vec![VecDeque::new(); instance.num_agents],
            graph: &instance.graph,
        }
    }

    pub fn get(&mut self, agent_id: usize, vertex_id: usize) -> isize {
        let max_dist = self.num_vertices as i32;
        if self.table[agent_id][vertex_id] < max_dist {
            return self.table[agent_id][vertex_id] as isize;
        }

        while let Some(current_vertex_id) = self.open_queues[agent_id].pop_front() {
            let current_dist = self.table[agent_id][current_vertex_id];

            for &neighbor_id in self.graph.get_neighbors(current_vertex_id) {
                let neighbor_dist = self.table[agent_id][neighbor_id];
                let new_dist = current_dist + 1;

                if new_dist < neighbor_dist {
                    self.table[agent_id][neighbor_id] = new_dist;
                    self.open_queues[agent_id].push_back(neighbor_id);
                }
            }

            if current_vertex_id == vertex_id {
                return current_dist as isize;
            }
        }

        panic!("unreachable vertex: agent={agent_id}, vertex={vertex_id}")
    }
}
