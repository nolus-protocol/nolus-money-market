use std::{array, mem};

use crate::compared_pair::{ComparedPairNotEq, SwapStatus};

use super::{super::CycleCreation, ConnectionIndexes, Graph};

mod edges;
mod vertices;

impl<Vertex, Edge> Graph<Vertex, Edge>
where
    Vertex: Ord,
{
    pub fn insert(
        &mut self,
        vertex_a: Vertex,
        edge_a_to_b: Edge,
        vertex_b: Vertex,
        edge_b_to_a: Edge,
    ) -> Result<Option<Relation<&'_ Vertex, Edge>>, CycleCreation> {
        ComparedPairNotEq::with_swap_status((vertex_a, edge_a_to_b), (vertex_b, edge_b_to_a))
            .ok_or(CycleCreation {})
            .map(|(vertices, swapped)| (self.get_or_insert_vertices(vertices), swapped))
            .and_then(|(vertices, swapped)| {
                let mut both_exist = true;

                let vertices = vertices.map_associated_values(|(exists, edge)| {
                    both_exist &= exists;

                    edge
                });

                if both_exist {
                    self.check_for_cycle(vertices.min().key(), vertices.max().key())
                        .map(|()| vertices)
                } else {
                    Ok(vertices)
                }
                .map(|vertices| {
                    self.create_or_replace_edge_unchecked(vertices)
                        .map(|mut relation| {
                            if matches!(swapped, SwapStatus::Swapped) {
                                mem::swap(&mut relation.vertex_a, &mut relation.vertex_b);

                                mem::swap(&mut relation.edge_a_to_b, &mut relation.edge_b_to_a);
                            }

                            relation
                        })
                })
            })
    }

    fn check_for_cycle(&mut self, vertex_a: usize, vertex_b: usize) -> Result<(), CycleCreation> {
        let mut trees = array::from_fn::<_, 2, _>({
            let capacity = if self.edges.is_empty() {
                0
            } else if let Ok(capacity) = self.edges.len().ilog2().try_into() {
                capacity
            } else {
                #[cold]
                fn unlikely() {}

                unlikely();

                unimplemented!(
                    "Base 2 logarithm of the length of edges cannot be fit within a `usize`!"
                )
            };

            move |_| Vec::with_capacity(capacity)
        });

        let mut visited = Vec::new();

        visited.reserve_exact(self.vertices.len());

        visited.resize(self.vertices.len(), false);

        [vertex_a, vertex_b]
            .into_iter()
            .enumerate()
            .for_each(|(tree_index, vertex)| {
                trees[tree_index].push([vertex, vertex]);

                visited[vertex] = true;
            });

        'cycle_detection: loop {
            for tree in trees.iter_mut() {
                if tree.is_empty() {
                    break 'cycle_detection Ok(());
                }

                for vertex in (0..tree.len()).rev() {
                    let [previous_vertex, vertex] = tree.swap_remove(vertex);

                    let edges = self.edges[self.edges_ranges[vertex].clone()]
                        .into_iter()
                        .filter_map(|&ConnectionIndexes { connected_to, .. }| {
                            (connected_to != previous_vertex).then_some(connected_to)
                        });

                    for connected_vertex in edges {
                        if mem::replace(&mut visited[connected_vertex], true) {
                            break 'cycle_detection Err(CycleCreation);
                        } else if connected_vertex != vertex {
                            tree.push([vertex, connected_vertex]);
                        }
                    }
                }
            }
        }
    }
}

pub struct Relation<Vertex, Edge> {
    pub vertex_a: Vertex,
    pub edge_a_to_b: Edge,
    pub vertex_b: Vertex,
    pub edge_b_to_a: Edge,
}
