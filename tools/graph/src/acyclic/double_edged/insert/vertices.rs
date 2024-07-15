use std::ops::Range;

use crate::compared_pair::ComparedPairNotEq;

use super::super::{ConnectionIndexes, Graph};

impl<Vertex, Edge> Graph<Vertex, Edge>
where
    Vertex: Ord,
{
    pub(super) fn get_or_insert_vertices(
        &mut self,
        vertices: ComparedPairNotEq<Vertex, Edge>,
    ) -> ComparedPairNotEq<usize, (bool, Edge)> {
        let (vertex_min, vertex_max) = vertices.into_entries();

        let [vertex_a, vertex_b] =
            [vertex_min.into_key_value(), vertex_max.into_key_value()].map(|(vertex, edge)| {
                let insert_result = self.get_or_insert_vertex(vertex);

                (insert_result.index, (insert_result.exists, edge))
            });

        let Some(vertices) = ComparedPairNotEq::new(vertex_a, vertex_b) else {
            unreachable!(
                "Indexes should be inherently not equal as the vertices are already checked!"
            )
        };

        // Assumption: The two indexes are checked for equality and ordered,
        //  thus the minimum key is only allowed values within `0..usize::MAX-1`
        //  and the maximum key is only allowed values between `1..usize::MAX`.
        debug_assert_ne!(vertices.min().key(), usize::MAX);
        debug_assert_ne!(vertices.max().key(), 0);
        debug_assert!(vertices.min().key() < vertices.max().key());

        match (
            vertices.min().associated_value().0,
            vertices.max().associated_value().0,
        ) {
            (false, false) => {
                let min_key = vertices.min().key();

                let max_key = vertices.max().key().wrapping_sub(1);

                self.edges
                    .iter_mut()
                    .filter_map(|ConnectionIndexes { connected_to, .. }| {
                        (min_key <= *connected_to).then_some(connected_to)
                    })
                    .for_each(|connected_to| {
                        *connected_to += if max_key <= *connected_to { 2 } else { 1 };
                    });
            }
            (min_exists @ true, false) | (min_exists @ false, true) => {
                let starting_from = if min_exists {
                    vertices.max().key()
                } else {
                    vertices.min().key()
                };

                self.edges
                    .iter_mut()
                    .filter_map(|ConnectionIndexes { connected_to, .. }| {
                        (starting_from <= *connected_to).then_some(connected_to)
                    })
                    .for_each(|connected_to| *connected_to += 1);
            }
            (true, true) => {}
        }

        vertices
    }

    fn get_or_insert_vertex(&mut self, vertex: Vertex) -> GetOrInsertResult {
        self.vertices
            .binary_search_by(|existing_vertex| existing_vertex.cmp(&vertex))
            .map_or_else(
                |index| {
                    self.insert_vertex_at(vertex, index);

                    GetOrInsertResult {
                        exists: false,
                        index,
                    }
                },
                |index| GetOrInsertResult {
                    exists: true,
                    index,
                },
            )
    }

    fn insert_vertex_at(&mut self, vertex: Vertex, index: usize) {
        self.vertices.insert(index, vertex);

        let connected_vertices_range = self
            .edges_ranges
            .get(index)
            .cloned()
            .map_or_else(|| self.edges.len(), |Range { start, .. }| start);

        self.edges_ranges
            .insert(index, connected_vertices_range..connected_vertices_range);
    }
}

struct GetOrInsertResult {
    exists: bool,
    index: usize,
}
