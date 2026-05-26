use std::{
    fs::File,
    io::{BufRead, BufReader},
};

use regex::Regex;

use crate::error::LacamError;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Vertex {
    pub id: usize,    // index for V in Graph
    pub index: usize, // index for U (width * y + x) in Graph
    pub x: i32,
    pub y: i32,
    pub neighbors: Vec<usize>,
    pub actions: Vec<usize>,
}

impl Vertex {
    pub fn new(id: usize, index: usize, x: i32, y: i32) -> Self {
        Self {
            id,
            index,
            x,
            y,
            neighbors: Vec::new(),
            actions: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Graph {
    pub vertices: Vec<Vertex>,    // V: all vertices
    pub grid: Vec<Option<usize>>, // U: grid with vertex IDs (None for obstacles)
    pub width: usize,
    pub height: usize,
}
impl Graph {
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
            grid: Vec::new(),
            width: 0,
            height: 0,
        }
    }

    pub fn from_file(filename: &str) -> crate::error::Result<Self> {
        let file = File::open(filename)?;
        let reader = BufReader::new(file);

        let height_regex = Regex::new(r"height\s+(\d+)").unwrap();
        let width_regex = Regex::new(r"width\s+(\d+)").unwrap();
        let map_regex = Regex::new(r"map").unwrap();

        let mut graph = Graph::new();
        let mut lines = reader.lines();

        while let Some(line) = lines.next() {
            let line = line?;
            let line = line.trim_end_matches('\r'); // CRLF

            if let Some(caps) = height_regex.captures(line) {
                graph.height = caps[1].parse().map_err(|_| LacamError::Parse {
                    field: "height".to_string(),
                })?;
            } else if let Some(caps) = width_regex.captures(line) {
                graph.width = caps[1].parse().map_err(|_| LacamError::Parse {
                    field: "width".to_string(),
                })?;
            } else if map_regex.is_match(line) {
                break;
            }
        }

        // Initialize grid
        graph.grid = vec![None; graph.width * graph.height];

        // Read map data
        let mut y = 0;
        while let Some(line) = lines.next() {
            let line = line?;
            let line = line.trim_end_matches('\r');

            for (x, ch) in line.chars().take(graph.width).enumerate() {
                if ch == 'T' || ch == '@' {
                    continue; // obstacle
                }

                let index = graph.width * y + x;
                let vertex_id = graph.vertices.len();
                let vertex = Vertex::new(vertex_id, index, x as i32, y as i32);

                graph.vertices.push(vertex);
                graph.grid[index] = Some(vertex_id);
            }
            y += 1;
        }

        for vertex_id in 0..graph.vertices.len() {
            let vertex = &graph.vertices[vertex_id];
            let x = vertex.x;
            let y = vertex.y;

            let mut neighbors = Vec::new();

            let directions = [(-1, 0), (1, 0), (0, 1), (0, -1)];

            for (dx, dy) in directions.iter() {
                let new_x = x + dx;
                let new_y = y + dy;

                if new_x >= 0
                    && new_x < graph.width as i32
                    && new_y >= 0
                    && new_y < graph.height as i32
                {
                    let neighbor_index = graph.width * (new_y as usize) + (new_x as usize);
                    if let Some(neighbor_id) = graph.grid[neighbor_index] {
                        neighbors.push(neighbor_id);
                    }
                }
            }

            // actions = neighbors + self
            let mut actions = neighbors.clone();
            actions.push(vertex_id);

            graph.vertices[vertex_id].neighbors = neighbors;
            graph.vertices[vertex_id].actions = actions;
        }

        Ok(graph)
    }

    pub fn from_array(grid: Vec<Vec<bool>>) -> crate::error::Result<Self> {
        let height = grid.len();
        let width = grid
            .first()
            .map(|row| row.len())
            .expect("unvalid map width");

        let mut g = Graph::new();
        g.height = height;
        g.width = width;
        g.grid = vec![None; width * height];

        for (y, row) in grid.iter().enumerate() {
            if row.len() != width {
                return Err(LacamError::Parse {
                    field: format!("grid row {} length {} != width {}", y, row.len(), width),
                });
            }
            for (x, &passable) in row.iter().enumerate() {
                if !passable {
                    continue;
                }
                let index = width * y + x;
                let vertex_id = g.vertices.len();
                g.vertices
                    .push(Vertex::new(vertex_id, index, x as i32, y as i32));
                g.grid[index] = Some(vertex_id)
            }
        }

        g.build_adjacency();

        Ok(g)
    }

    fn build_adjacency(&mut self) {
        for vertex_id in 0..self.vertices.len() {
            let vertex = &self.vertices[vertex_id];
            let x = vertex.x;
            let y = vertex.y;

            let mut neighbors = Vec::new();
            let directions = [(-1, 0), (1, 0), (0, 1), (0, -1)];

            for (dx, dy) in directions.iter() {
                let new_x = x + dx;
                let new_y = y + dy;

                if new_x >= 0
                    && new_x < self.width as i32
                    && new_y >= 0
                    && new_y < self.height as i32
                {
                    let neighbor_index = self.width * (new_y as usize) + (new_x as usize);
                    if let Some(neighbor_id) = self.grid[neighbor_index] {
                        neighbors.push(neighbor_id);
                    }
                }
            }

            let mut actions = neighbors.clone();
            actions.push(vertex_id);

            self.vertices[vertex_id].neighbors = neighbors;
            self.vertices[vertex_id].actions = actions;
        }
    }

    pub fn size(&self) -> usize {
        self.vertices.len()
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn get_vertex(&self, id: usize) -> Option<&Vertex> {
        self.vertices.get(id)
    }

    pub fn get_neighbors(&self, vertex_id: usize) -> &[usize] {
        if let Some(vertex) = self.get_vertex(vertex_id) {
            &vertex.neighbors
        } else {
            &[]
        }
    }

    pub fn get_actions(&self, vertex_id: usize) -> &[usize] {
        if let Some(vertex) = self.get_vertex(vertex_id) {
            &vertex.actions
        } else {
            &[]
        }
    }
}

pub type Config = Vec<usize>; // config[agent_id] = vertex_id
