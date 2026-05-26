use std::collections::VecDeque;

use crate::{graph::Graph, instance::Instance};

#[derive(Debug, Clone)]
pub struct DistTable<'a> {
    pub num_vertices: usize,
    pub table: Vec<Vec<isize>>,
    pub open_queues: Vec<VecDeque<usize>>,
    graph: &'a Graph,
}

impl<'a> DistTable<'a> {
    pub fn new_lazy(instance: &'a Instance) -> Self {
        let max_dist = instance.graph.vertices.len() as isize;
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

    pub fn get(&mut self, agent_id: usize, vertex_id: usize) -> isize {
        let max_dist = self.num_vertices as isize;
        if self.table[agent_id][vertex_id] < max_dist {
            return self.table[agent_id][vertex_id];
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
                return current_dist;
            }
        }

        panic!("unreachable vertex: agent={agent_id}, vertex={vertex_id}")
    }
}
