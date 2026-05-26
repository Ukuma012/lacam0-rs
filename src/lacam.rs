use std::collections::{BTreeSet, HashMap, VecDeque};

use rand::{Rng, SeedableRng, rngs::StdRng};

use crate::{deadline::Deadline, dist_table::DistTable, graph::Config, instance::Instance, pibt::{PIBT, PartialConfig}};

pub type Solution = Vec<Config>;

#[derive(Debug)]
pub enum SolveOutcome {
    Solved(Solution),
    Timeout,
    NoSolution,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct HNodeId(usize);

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct LNodeId(usize);

pub struct HNode {
    pub config: Config,
    pub parent: Option<HNodeId>,
    pub neighbors: BTreeSet<HNodeId>,

    pub g: isize,
    pub h: isize,
    pub f: isize,

    pub priorities: Vec<f32>,
    pub order: Vec<usize>,

    pub search_tree: VecDeque<LNodeId>,
}

pub struct LNode {
    // (agent_id, vertex_id) pairs; constraints.len() is the depth
    pub constraints: Vec<(usize, usize)>,
}

pub struct HNodeArena {
    nodes: Vec<HNode>,
}

impl Default for HNodeArena {
    fn default() -> Self {
        Self { nodes: Vec::new() }
    }
}

impl HNodeArena {
    pub fn alloc(&mut self, node: HNode) -> HNodeId {
        let id = HNodeId(self.nodes.len());
        self.nodes.push(node);
        id
    }

    pub fn get(&self, id: HNodeId) -> &HNode {
        &self.nodes[id.0]
    }

    pub fn get_mut(&mut self, id: HNodeId) -> &mut HNode {
        &mut self.nodes[id.0]
    }
}

pub struct LNodeArena {
    nodes: Vec<LNode>,
}

impl Default for LNodeArena {
    fn default() -> Self {
        Self { nodes: Vec::new() }
    }
}

impl LNodeArena {
    pub fn alloc(&mut self, node: LNode) -> LNodeId {
        let id = LNodeId(self.nodes.len());
        self.nodes.push(node);
        id
    }

    pub fn get(&self, id: LNodeId) -> &LNode {
        &self.nodes[id.0]
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }
}

pub struct LaCAM<'a> {
    pub instance: &'a Instance,
    pub dist_table: DistTable<'a>,
    pub deadline: Deadline,
    pub pibt: PIBT<'a>,

    pub h_arena: HNodeArena,
    pub l_arena: LNodeArena,

    pub open: VecDeque<HNodeId>,
    pub explored: HashMap<Config, HNodeId>,
    pub h_goal: Option<HNodeId>,
    pub rng: StdRng,

    pub loop_cnt: usize,
    pub flg_star: bool,
}

impl<'a> LaCAM<'a> {
    pub fn new(instance: &'a Instance, dist_table: DistTable<'a>, deadline: Deadline, seed: u64, flg_star: bool) -> Self {
        Self {
            instance,
            deadline,
            dist_table,
            pibt: PIBT::new(instance, seed),
            h_arena: HNodeArena::default(),
            l_arena: LNodeArena::default(),
            open: VecDeque::new(),
            explored: HashMap::new(),
            h_goal: None,
            rng: StdRng::seed_from_u64(seed),
            loop_cnt: 0,
            flg_star 
        }
    }

    fn get_h_val(&mut self, config: &Config) -> isize {
        let mut cost = 0;
        for i in 0..self.instance.num_agents {
            cost += self.dist_table.get(i, config[i]);
        }

        cost
    }

    fn get_edge_cost(&self, c1: &Config, c2: &Config) -> isize {
        let mut cost = 0;
        for i in 0..self.instance.num_agents {
            if c1[i] != self.instance.goals[i] || c2[i] != self.instance.goals[i] {
                cost += 1;
            }
        }
        cost
    }

    fn create_highlevel_node(&mut self, config: &Config, parent: Option<HNodeId>) -> HNodeId {
        let n = self.instance.num_agents;

        let g = match parent {
            None => 0,
            Some(pid) => {
                let parent_config = self.h_arena.get(pid).config.clone();
                let parent_g = self.h_arena.get(pid).g;
                parent_g + self.get_edge_cost(&parent_config, config)
            }
        };

        // h: heuristic
        let h = self.get_h_val(config);
        let f = g + h;

        // priorities
        let priorities = match parent {
            None => {
                // initial node
                let mut p = vec![0.0f32; n];
                for i in 0..n {
                    let d = self.dist_table.get(i, config[i]);
                    p[i] = d as f32 / 10000.0;
                }
                p
            }
            Some(pid) => {
                let parent_priorities = self.h_arena.get(pid).priorities.clone();
                let mut p = vec![0.0f32; n];
                for i in 0..n {
                    let d = self.dist_table.get(i, config[i]);
                    if d != 0 {
                        p[i] = parent_priorities[i] + 1.0;
                    } else {
                        // 3.7 - 3.0 = 0.7
                        p[i] = parent_priorities[i] - parent_priorities[i].floor();
                    }
                }
                p
            }
        };

        // order
        let mut order: Vec<usize> = (0..n).collect();
        order.sort_by(|&a, &b| {
            priorities[b]
                .partial_cmp(&priorities[a])
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let l_init = self.l_arena.alloc(LNode {
            constraints: Vec::new(),
        });
        let mut search_tree = VecDeque::new();
        search_tree.push_back(l_init);

        let h_id = self.h_arena.alloc(HNode {
            config: config.clone(),
            parent,
            neighbors: BTreeSet::new(),
            g,
            h,
            f,
            priorities,
            order,
            search_tree,
        });

        if let Some(pid) = parent {
            self.h_arena.get_mut(h_id).neighbors.insert(pid);
            self.h_arena.get_mut(pid).neighbors.insert(h_id);
        }

        self.explored.insert(config.clone(), h_id);

        h_id
    }

        fn get_next_lowlevel_node(&mut self, h_id: HNodeId) -> Option<LNodeId> {
        if self.h_arena.get(h_id).search_tree.is_empty() {
            return None;
        }

        let l_id = self.h_arena.get_mut(h_id).search_tree.pop_front().unwrap();
        let depth = self.l_arena.get(l_id).constraints.len();
        let n = self.instance.num_agents;

        if depth < n {
            let parent_constraints = self.l_arena.get(l_id).constraints.clone();
            let i = self.h_arena.get(h_id).order[depth];
            let current_v = self.h_arena.get(h_id).config[i];

            let mut cands: Vec<usize> = self.instance.graph.get_neighbors(current_v).to_vec();
            cands.push(current_v);

            for k in (1..cands.len()).rev() {
                let j = self.rng.random_range(0..=k);
                cands.swap(k, j);
            }

            for &u in &cands {
                let mut new_constraints = parent_constraints.clone();
                new_constraints.push((i, u));

                let child_id = self.l_arena.alloc(LNode {
                    constraints: new_constraints,
                });
                self.h_arena.get_mut(h_id).search_tree.push_back(child_id);
            }
        }

        Some(l_id)
    }

    pub fn solve(&mut self) -> SolveOutcome {
        let num_agents = self.instance.num_agents;
        let starts = self.instance.starts.clone();

        // initial node
        let h_init = self.create_highlevel_node(&starts, None);
        self.open.push_front(h_init);

        let mut expired = false;

        // search loop
        while !self.open.is_empty() {
            if self.deadline.is_expired() {
                expired = true;
                break;
            }

            self.loop_cnt += 1;

            // @TODO random insert

            let h_id = *self.open.front().unwrap();

            if let Some(goal_id) = self.h_goal {
                if self.h_arena.get(h_id).f >= self.h_arena.get(goal_id).f {
                    self.open.pop_front();
                    continue;
                }
            }

            if self.h_goal.is_none() && self.h_arena.get(h_id).config == self.instance.goals {
                self.h_goal = Some(h_id);
                if !self.flg_star {
                    break;
                }
                continue;
            }

            let l_id = match self.get_next_lowlevel_node(h_id) {
                Some(id) => id,
                None => {
                    self.open.pop_front();
                    continue;
                }
            };

            let q_from = self.h_arena.get(h_id).config.clone();
            let order = self.h_arena.get(h_id).order.clone();

            let mut q_to: PartialConfig = vec![None; num_agents];
            for &(agent, vertex) in &self.l_arena.get(l_id).constraints {
                q_to[agent] = Some(vertex);
            }

            if !self.pibt.set_new_config(&q_from, &mut q_to, order, &mut self.dist_table) {
                continue; // failed -> next
            }

            let q_to: Config = q_to.into_iter().map(|v| v.unwrap()).collect();

            if let Some(&existing_id) = self.explored.get(&q_to) {
                self.rewrite(h_id, existing_id);
                self.open.push_front(existing_id);
            } else {
                let h_new = self.create_highlevel_node(&q_to, Some(h_id));
                self.open.push_front(h_new);
            }
        }

        match (self.h_goal, expired) {
            (Some(h_id), _) => SolveOutcome::Solved(self.backtrack(h_id)),
            (None, false) => SolveOutcome::NoSolution,
            (None, true) => SolveOutcome::Timeout,
        }
    }

    fn backtrack(&self, h_id: HNodeId) -> Solution {
        let mut plan = Vec::new();
        let mut current = Some(h_id);
        while let Some(id) = current {
            let node = self.h_arena.get(id);
            plan.push(node.config.clone());
            current = node.parent;
        }
        plan.reverse();
        plan
    }

    fn rewrite(&mut self, from_id: HNodeId, to_id: HNodeId) {
        self.h_arena.get_mut(from_id).neighbors.insert(to_id);

        let mut queue: VecDeque<HNodeId> = VecDeque::new();
        queue.push_back(from_id);

        while let Some(n_from) = queue.pop_front() {
            let from_g = self.h_arena.get(n_from).g;
            let from_config = self.h_arena.get(n_from).config.clone();
            let neighbors: Vec<HNodeId> =
                self.h_arena.get(n_from).neighbors.iter().copied().collect();

            for n_to in neighbors {
                let to_config = self.h_arena.get(n_to).config.clone();
                let new_g = from_g + self.get_edge_cost(&from_config, &to_config);

                if new_g < self.h_arena.get(n_to).g {
                    let to_h = self.h_arena.get(n_to).h;
                    let node = self.h_arena.get_mut(n_to);
                    node.g = new_g;
                    node.f = new_g + to_h;
                    node.parent = Some(n_from);

                    queue.push_back(n_to);

                    if let Some(goal_id) = self.h_goal {
                        if self.h_arena.get(n_to).f < self.h_arena.get(goal_id).f {
                            self.open.push_front(n_to);
                        }
                    }
                }
            }
        }
    }
}