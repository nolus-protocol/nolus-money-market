use std::{
    borrow::Borrow,
    fmt::{self, Debug, Formatter},
    ops::Range,
};

pub use self::{connected_vertices::ConnectedVertices, insert::Relation};

mod connected_vertices;
mod insert;

pub struct Graph<Vertex, Edge> {
    vertices: Vec<Vertex>,
    edges_ranges: Vec<Range<usize>>,
    edges: Vec<ConnectionIndexes>,
    edge_values: Vec<Edge>,
}

impl<Vertex, Edge> Graph<Vertex, Edge> {
    const EMPTY: Self = Self {
        vertices: Vec::new(),
        edges_ranges: Vec::new(),
        edges: Vec::new(),
        edge_values: Vec::new(),
    };

    pub const fn new() -> Self {
        Self::EMPTY
    }

    pub fn get_edge<'r, 't, T>(&'r self, from: &'t T, to: &'t T) -> Option<&'r Edge>
    where
        Vertex: Borrow<T>,
        T: Ord + ?Sized,
    {
        self.get_edge_index(from, to)
            .map(|edge| &self.edge_values[edge])
    }

    pub fn get_edge_mut<'r, 't, T>(&'r mut self, from: &'t T, to: &'t T) -> Option<&'r mut Edge>
    where
        Vertex: Borrow<T>,
        T: Ord + ?Sized,
    {
        self.get_edge_index(from, to)
            .map(|edge| &mut self.edge_values[edge])
    }

    pub fn connected_vertices<'r, 't, T>(
        &'r self,
        from: &'t T,
    ) -> Option<ConnectedVertices<'r, Vertex, Edge>>
    where
        Vertex: Borrow<T>,
        T: Ord + ?Sized,
    {
        self.vertices
            .binary_search_by(|vertex| vertex.borrow().cmp(from))
            .ok()
            .map(|vertex| ConnectedVertices::new(self, vertex))
    }

    fn get_edge_index<T>(&self, from: &T, to: &T) -> Option<usize>
    where
        Vertex: Borrow<T>,
        T: Ord + ?Sized,
    {
        self.vertices
            .binary_search_by(|vertex| vertex.borrow().cmp(from))
            .ok()
            .and_then(|from| {
                self.vertices
                    .binary_search_by(|vertex| vertex.borrow().cmp(to))
                    .ok()
                    .map(|to| (from, to))
            })
            .and_then(|(from, to)| {
                let edges = &self.edges[self.edges_ranges[from].clone()];

                edges
                    .binary_search_by(|&ConnectionIndexes { connected_to, .. }| {
                        connected_to.cmp(&to)
                    })
                    .ok()
                    .map(|edge| edges[edge].edge_value)
            })
    }
}

impl<Vertex, Edge> Debug for Graph<Vertex, Edge>
where
    Vertex: Debug,
    Edge: Debug,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_list()
            .entries(
                self.vertices
                    .iter()
                    .zip(self.edges_ranges.iter().cloned())
                    .flat_map(|(left, range)| {
                        self.edges[range].iter().map(
                            move |&ConnectionIndexes {
                                      connected_to,
                                      edge_value: edge,
                                  }| {
                                (left, &self.edge_values[edge], &self.vertices[connected_to])
                            },
                        )
                    }),
            )
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ConnectionIndexes {
    connected_to: usize,
    edge_value: usize,
}

#[cfg(test)]
fn well_known_graph() -> Graph<&'static str, String> {
    Graph {
        vertices: vec![
            "v0", "v1", "v10", "v11", "v12", "v13", "v14", "v15", "v2", "v3", "v4", "v5", "v6",
            "v7", "v8", "v9",
        ],
        edges_ranges: vec![
            0..2,
            2..3,
            3..5,
            5..6,
            6..10,
            10..11,
            11..12,
            12..13,
            13..16,
            16..18,
            18..19,
            19..22,
            22..23,
            23..25,
            25..27,
            27..30,
        ],
        edges: vec![
            ConnectionIndexes {
                connected_to: 8,
                edge_value: 4,
            },
            ConnectionIndexes {
                connected_to: 10,
                edge_value: 6,
            },
            ConnectionIndexes {
                connected_to: 8,
                edge_value: 0,
            },
            ConnectionIndexes {
                connected_to: 4,
                edge_value: 26,
            },
            ConnectionIndexes {
                connected_to: 15,
                edge_value: 12,
            },
            ConnectionIndexes {
                connected_to: 15,
                edge_value: 22,
            },
            ConnectionIndexes {
                connected_to: 2,
                edge_value: 27,
            },
            ConnectionIndexes {
                connected_to: 5,
                edge_value: 18,
            },
            ConnectionIndexes {
                connected_to: 6,
                edge_value: 14,
            },
            ConnectionIndexes {
                connected_to: 7,
                edge_value: 20,
            },
            ConnectionIndexes {
                connected_to: 4,
                edge_value: 19,
            },
            ConnectionIndexes {
                connected_to: 4,
                edge_value: 15,
            },
            ConnectionIndexes {
                connected_to: 4,
                edge_value: 21,
            },
            ConnectionIndexes {
                connected_to: 0,
                edge_value: 5,
            },
            ConnectionIndexes {
                connected_to: 1,
                edge_value: 1,
            },
            ConnectionIndexes {
                connected_to: 9,
                edge_value: 2,
            },
            ConnectionIndexes {
                connected_to: 8,
                edge_value: 3,
            },
            ConnectionIndexes {
                connected_to: 11,
                edge_value: 28,
            },
            ConnectionIndexes {
                connected_to: 0,
                edge_value: 7,
            },
            ConnectionIndexes {
                connected_to: 9,
                edge_value: 29,
            },
            ConnectionIndexes {
                connected_to: 12,
                edge_value: 8,
            },
            ConnectionIndexes {
                connected_to: 13,
                edge_value: 10,
            },
            ConnectionIndexes {
                connected_to: 11,
                edge_value: 9,
            },
            ConnectionIndexes {
                connected_to: 11,
                edge_value: 11,
            },
            ConnectionIndexes {
                connected_to: 14,
                edge_value: 24,
            },
            ConnectionIndexes {
                connected_to: 13,
                edge_value: 25,
            },
            ConnectionIndexes {
                connected_to: 15,
                edge_value: 16,
            },
            ConnectionIndexes {
                connected_to: 2,
                edge_value: 13,
            },
            ConnectionIndexes {
                connected_to: 3,
                edge_value: 23,
            },
            ConnectionIndexes {
                connected_to: 14,
                edge_value: 17,
            },
        ],
        edge_values: [
            "e-v1-v2",   // 1
            "e-v2-v1",   // 1
            "e-v2-v3",   // 2
            "e-v3-v2",   // 2
            "e-v0-v2",   // 3
            "e-v2-v0",   // 3
            "e-v0-v4",   // 5
            "e-v4-v0",   // 5
            "e-v5-v6",   // 8
            "e-v6-v5",   // 8
            "e-v5-v7",   // 9
            "e-v7-v5",   // 9
            "e-v10-v9",  // 10
            "e-v9-v10",  // 10
            "e-v12-v14", // 11
            "e-v14-v12", // 11
            "e-v8-v9",   // 12
            "e-v9-v8",   // 12
            "e-v12-v13", // 13
            "e-v13-v12", // 13
            "e-v12-v15", // 14
            "e-v15-v12", // 14
            "e-v11-v9",  // 15
            "e-v9-v11",  // 15
            "e-v7-v8",   // 16
            "e-v8-v7",   // 16
            "e-v10-v12", // 17
            "e-v12-v10", // 17
            "e-v3-v5",   // 18
            "e-v5-v3",   // 18
        ]
        .map(String::from)
        .to_vec(),
    }
}

#[test]
fn test_graph_construction_against_well_known() {
    use super::CycleCreation;

    let edges = [
        ("v1", "v2", true),   // 1
        ("v2", "v3", true),   // 2
        ("v2", "v0", true),   // 3
        ("v0", "v1", false),  // 4
        ("v4", "v0", true),   // 5
        ("v3", "v4", false),  // 6
        ("v4", "v2", false),  // 7
        ("v5", "v6", true),   // 8
        ("v7", "v5", true),   // 9
        ("v9", "v10", true),  // 10
        ("v12", "v14", true), // 11
        ("v9", "v8", true),   // 12
        ("v12", "v13", true), // 13
        ("v12", "v15", true), // 14
        ("v9", "v11", true),  // 15
        ("v7", "v8", true),   // 16
        ("v12", "v10", true), // 17
        ("v3", "v5", true),   // 18
        ("v15", "v1", false), // 19
    ];

    let mut graph = Graph::new();

    edges
        .into_iter()
        .enumerate()
        .for_each(|(n, (a, b, is_ok))| {
            let result = graph
                .insert(a, format!("e-{a}-{b}"), b, format!("e-{b}-{a}"))
                .map(drop);

            assert!(
                if is_ok {
                    matches!(result, Ok(()))
                } else {
                    matches!(result, Err(CycleCreation {}))
                },
                "n={n}, a={a}, b={b}",
            );
        });

    let well_known_graph = well_known_graph();

    assert_eq!(graph.vertices, well_known_graph.vertices);

    assert_eq!(graph.edges_ranges, well_known_graph.edges_ranges);

    assert_eq!(graph.edges, well_known_graph.edges);

    assert_eq!(graph.edge_values, well_known_graph.edge_values);
}
