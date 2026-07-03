use rand::{Rng, SeedableRng, rngs::StdRng};

use crate::{dist_table::DistTable, graph::Config, instance::Instance};

pub type PartialConfig = Vec<Option<usize>>;

#[derive(Clone, Copy, PartialEq, PartialOrd)]
pub struct PIBTHeuristic {
    pub dist: isize,
    pub hindrance: usize,
    pub tie: f64,
}

impl Default for PIBTHeuristic {
    fn default() -> Self {
        Self {
            dist: 0,
            hindrance: 0,
            tie: 0.0,
        }
    }
}

impl Ord for PIBTHeuristic {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.dist.cmp(&other.dist).then(
            self.hindrance.cmp(&other.hindrance).then(
                self.tie
                    .partial_cmp(&other.tie)
                    .unwrap_or(std::cmp::Ordering::Equal),
            ),
        )
    }
}
impl Eq for PIBTHeuristic {}

pub struct PIBT<'a> {
    pub instance: &'a Instance,
    pub rng: StdRng,
    pub num_agents: usize,
    pub occupied_now: Vec<Option<usize>>,
    pub occupied_next: Vec<Option<usize>>,
    pub c_next: Vec<[usize; 5]>,
    pub c_cost: [PIBTHeuristic; 5],
    pub c_indices: Vec<[usize; 5]>,
    pub swap: bool,
    pub hindrance: bool,
}

impl<'a> PIBT<'a> {
    pub fn new(instance: &'a Instance, seed: u64) -> Self {
        let num_agents = instance.num_agents;
        let num_vertices = instance.graph.size();

        Self {
            instance,
            rng: StdRng::seed_from_u64(seed),
            num_agents,
            occupied_now: vec![None; num_vertices],
            occupied_next: vec![None; num_vertices],
            c_next: vec![[0; 5]; num_agents],
            c_cost: [PIBTHeuristic::default(); 5],
            c_indices: vec![[0; 5]; num_agents],
            swap: true,
            hindrance: true,
        }
    }

    pub fn set_new_config(
        &mut self,
        q_from: &Config,
        q_to: &mut PartialConfig,
        order: Vec<usize>,
        dist_table: &mut DistTable<'a>,
    ) -> bool {
        let mut success = true;

        for i in 0..self.num_agents { 
            self.occupied_now[q_from[i]] = Some(i);

            // set occupied next
            if let Some(vertex_id) = q_to[i] {
                // vertex collision
                if self.occupied_next[vertex_id].is_some() {
                    success = false;
                    break;
                }

                // swap collision
                if let Some(j) = self.occupied_now[vertex_id] {
                    if j != i {
                        if q_to[j] == Some(q_from[i]) {
                            success = false;
                            break;
                        }
                    }
                }

                self.occupied_next[vertex_id] = Some(i);
            }
        }

        if success {
            for &i in order.iter() {
                if q_to[i].is_none() && !self.func_pibt(i, q_from, q_to, dist_table) {
                    success = false;
                    break;
                }
            }
        }

        // cleanup
        for i in 0..self.num_agents {
            self.occupied_now[q_from[i]] = None;
            if let Some(vertex_id) = q_to[i] {
                self.occupied_next[vertex_id] = None;
            }
        }

        success
    }

    fn func_pibt(&mut self, agent_id: usize, q_from: &Config, q_to: &mut PartialConfig, dist_table: &mut DistTable<'a>) -> bool {
        let current_vertex_id = q_from[agent_id];
        let actions = self.instance.graph.get_actions(current_vertex_id);
        let neighbors = self.instance.graph.get_neighbors(current_vertex_id);

        // hindrance preparation
        let mut num_neighbor_agents = 0;
        let mut neighbor_agents = [0; 4];

        if self.hindrance {
            for &neighbor_id in neighbors {
                if let Some(agent_id) = self.occupied_now[neighbor_id] {
                    neighbor_agents[num_neighbor_agents] = agent_id;
                    num_neighbor_agents += 1;
                }
            }
        }

        for (k, &vertex_id) in actions.iter().enumerate() {
            self.c_next[agent_id][k] = vertex_id;

            let tie = self.rng.random::<f64>();
            let dist = dist_table.get(agent_id, vertex_id);

            let mut hindrance = 0;
            if self.hindrance {
                for i in 0..num_neighbor_agents {
                    let j = neighbor_agents[i];
                    let current_j = q_from[j];

                    if current_j != vertex_id {
                        let dist_j_to_vertex = dist_table.get(j, vertex_id);
                        let dist_j_to_current = dist_table.get(j, current_j);

                        if dist_j_to_vertex < dist_j_to_current {
                            hindrance += 1;
                        }
                    }
                }
            }

            self.c_cost[k] = PIBTHeuristic {
                dist,
                hindrance,
                tie,
            }
        }

        let k = actions.len();

        for i in 0..k {
            self.c_indices[agent_id][i] = i;
        }

        self.c_indices[agent_id][0..k].sort_by(|&a, &b| self.c_cost[a].cmp(&self.c_cost[b]));

        // emulate swap
        let best_candidate_vertex_id = self.c_next[agent_id][self.c_indices[agent_id][0]];
        let swap_agent =
            self.is_swap_required_and_possible(agent_id, q_from, q_to, best_candidate_vertex_id, dist_table);

        if swap_agent.is_some() {
            for i in 0..k {
                let vertex_id = self.c_next[agent_id][i];
                let tie = self.rng.random::<f64>();
                let dist = dist_table.get(agent_id, vertex_id);

                self.c_cost[i] = PIBTHeuristic {
                    dist: -dist,
                    hindrance: 0,
                    tie,
                };

                self.c_indices[agent_id][i] = i;
            }

            // re-sort
            self.c_indices[agent_id][0..k].sort_by(|&a, &b| self.c_cost[a].cmp(&self.c_cost[b]));
        }

        // main loop
        for i in 0..k {
            let u_idx = self.c_indices[agent_id][i];
            let vertex_id = self.c_next[agent_id][u_idx];

            if self.occupied_next[vertex_id].is_some() {
                continue;
            }

            let j = self.occupied_now[vertex_id];

            if let Some(j_id) = j {
                if q_to[j_id] == Some(q_from[agent_id]) {
                    continue;
                }
            }

            self.occupied_next[vertex_id] = Some(agent_id);
            q_to[agent_id] = Some(vertex_id);

            // priority inheritance
            if let Some(j_id) = j {
                let current_vertex_id = q_from[agent_id];
                if vertex_id != current_vertex_id && q_to[j_id].is_none() {
                    if !self.func_pibt(j_id, q_from, q_to, dist_table) {
                        // failed to move agent j, try next candidate
                        continue;
                    }
                }
            }

            if i == 0 {
                // execute swap operation
                if let Some(swap_agent_id) = swap_agent {
                    let current_vertex_id = q_from[agent_id];
                    if q_to[swap_agent_id].is_none()
                        && self.occupied_next[current_vertex_id].is_none()
                    {
                        self.occupied_next[current_vertex_id] = Some(swap_agent_id);
                        q_to[swap_agent_id] = Some(current_vertex_id)
                    }
                }
            }
            return true;
        }

        let current_vertex_id = q_from[agent_id];
        self.occupied_next[current_vertex_id] = Some(agent_id);
        q_to[agent_id] = Some(current_vertex_id);

        return false;
    }

    fn is_swap_required_and_possible(
        &mut self,
        agent_id: usize,
        q_from: &Config,
        q_to: &mut PartialConfig,
        vertex_id: usize,
        dist_table: &mut DistTable<'a>,
    ) -> Option<usize> {
        if !self.swap {
            return None;
        }

        // agent wants to stay -> no need to swap
        if vertex_id == q_from[agent_id] {
            return None;
        }

        // usual swap situation 
        if let Some(j) = self.occupied_now[vertex_id] {
            let v_i = q_from[agent_id];
            let v_j = q_from[j];
            if j != agent_id
                && q_to[j].is_none()
                && self.is_swap_required(agent_id, j, v_i, v_j, dist_table)
                && self.is_swap_possible(v_j, v_i)
            {
                return Some(j);
            }
        }

        // for clear operation; checked even when the best vertex is occupied
        let current_vertex_id = q_from[agent_id];
        let neighbors = self.instance.graph.get_neighbors(current_vertex_id);

        for &u in neighbors {
            if let Some(k) = self.occupied_now[u] {
                if k != agent_id
                    && vertex_id != q_from[k]
                    && self.is_swap_required(k, agent_id, current_vertex_id, vertex_id, dist_table)
                    && self.is_swap_possible(vertex_id, current_vertex_id)
                {
                    return Some(k);
                }
            }
        }

        None // no need to swap
    }

    fn is_swap_required(&mut self, pusher: usize, puller: usize, v_pusher_origin: usize, v_puller_origin: usize, dist_table: &mut DistTable<'a>) -> bool {
        let mut v_pusher = v_pusher_origin;
        let mut v_puller = v_puller_origin;
        let mut tmp = None;

        while dist_table.get(pusher, v_puller)
            < dist_table.get(pusher, v_pusher)
        {
            let neighbors = self.instance.graph.get_neighbors(v_puller);
            let mut n = neighbors.len();

            for &u in neighbors {
                let i = self.occupied_now[u];

                if u == v_pusher
                    || (self.instance.graph.get_neighbors(u).len() == 1
                        && i.is_some()
                        && self.instance.goals[i.unwrap()] == u)
                {
                    n -= 1;
                } else {
                    tmp = Some(u);
                }
            }

            if n >= 2 {
                return false;
            }

            if n <= 0 {
                break;
            }

            v_pusher = v_puller;
            v_puller = tmp.unwrap();
        }

        let puller_prefers_pusher = dist_table.get(puller, v_pusher)
            < dist_table.get(puller, v_puller);

        let pusher_at_goal_or_benefits = dist_table.get(pusher, v_pusher) == 0
            || dist_table.get(pusher, v_puller)
                < dist_table.get(pusher, v_pusher);

        puller_prefers_pusher && pusher_at_goal_or_benefits
    }

    fn is_swap_possible(&self, v_pusher_origin: usize, v_puller_origin: usize) -> bool {
        let mut v_pusher = v_pusher_origin;
        let mut v_puller = v_puller_origin;
        let mut tmp = None;

        while v_puller != v_pusher_origin {
            let neighbors = self.instance.graph.get_neighbors(v_puller);
            let mut n = neighbors.len();

            for &u in neighbors {
                let i = self.occupied_now[u];
                if u == v_pusher
                    || (self.instance.graph.get_neighbors(u).len() == 1
                        && i.is_some()
                        && self.instance.goals[i.unwrap()] == u)
                {
                    n -= 1;
                } else {
                    tmp = Some(u);
                }
            }
            if n >= 2 {
                return true; // able to swap at v_next
            }

            if n <= 0 {
                return false;
            }

            v_pusher = v_puller;
            v_puller = tmp.unwrap();
        }

        false
    }
}