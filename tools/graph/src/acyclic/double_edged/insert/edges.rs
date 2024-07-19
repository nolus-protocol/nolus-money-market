use std::{convert::identity, mem, ops::Range};

use crate::compared_pair::ComparedPairNotEq;

use super::super::{ConnectionIndexes, Graph, Relation};

impl<Vertex, Edge> Graph<Vertex, Edge>
where
    Vertex: Ord,
{
    /// # Safety
    /// * The edge between the two vertices must not form a cycle.
    pub(super) fn create_or_replace_edge_unchecked(
        &mut self,
        vertices: ComparedPairNotEq<usize, Edge>,
    ) -> Option<Relation<&Vertex, Edge>> {
        let vertex_min = vertices.min().key();

        let vertex_max = vertices.max().key();

        debug_assert!(
            self.check_for_cycle(vertex_min, vertex_max).is_err(),
            "Cycle creation detected!"
        );

        let (edge_min_exists, edge_min) = {
            let edges_range_min = &self.edges_ranges[vertex_min];

            let result = self.edges[edges_range_min.clone()].binary_search_by({
                let vertex_max = &vertex_max;

                move |&ConnectionIndexes { connected_to, .. }| connected_to.cmp(vertex_max)
            });

            (
                result.is_ok(),
                edges_range_min.start + result.unwrap_or_else(identity),
            )
        };

        if edge_min_exists {
            let Ok(edge_max) = ({
                let edges_range_max = &self.edges_ranges[vertex_max];

                self.edges[edges_range_max.clone()]
                    .binary_search_by({
                        move |&ConnectionIndexes { connected_to, .. }| connected_to.cmp(&vertex_min)
                    })
                    .map(|edge_max| edges_range_max.start + edge_max)
            }) else {
                unreachable!("Edge from maximum vertex to minimum vertex should exist, if the reverse exists!")
            };

            Some(self.replace_edges(vertices.map_associated_values_detached(
                |edge_value_min| (edge_min, edge_value_min),
                |edge_value_max| (edge_max, edge_value_max),
            )))
        } else {
            let (vertices, (edge_value_min, edge_value_max)) = vertices.take_associated_values();

            () = self.create_edge_from_min_unchecked(vertices, edge_min, edge_value_min);

            // Safety: The connection cannot exist, if the connection from the
            //  first vertex to the second vertex doesn't.
            () = self.create_edge_from_max_unchecked(vertices, edge_value_max);

            None
        }
    }

    fn replace_edges(
        &mut self,
        vertices: ComparedPairNotEq<usize, (usize, Edge)>,
    ) -> Relation<&Vertex, Edge> {
        let (vertex_min, vertex_max) = vertices.into_entries();

        let (vertex_min, (edge_min, edge_value_min)) = vertex_min.into_key_value();

        let (vertex_max, (edge_max, edge_value_max)) = vertex_max.into_key_value();

        Relation {
            vertex_a: &self.vertices[vertex_min],
            edge_a_to_b: mem::replace(
                &mut self.edge_values[self.edges[edge_min].edge_value],
                edge_value_min,
            ),
            vertex_b: &self.vertices[vertex_max],
            edge_b_to_a: mem::replace(
                &mut self.edge_values[self.edges[edge_max].edge_value],
                edge_value_max,
            ),
        }
    }

    /// Creates edge and links the minimum vertex to the maximum vertex, while
    /// also adjusting all indexes up to the maximum vertex, included.
    ///
    /// # Safety
    /// * The minimum vertex edge, `edge_min`, to the maximum vertex must not
    ///   already exist.
    /// * This method adjusts indexes up to the maximum vertex, included. It
    ///   needs to be paired with [`Self::create_edge_from_max_unchecked`] in
    ///   order to adjust the rest of the indexes.
    fn create_edge_from_min_unchecked(
        &mut self,
        vertices: ComparedPairNotEq<usize, ()>,
        edge_min: usize,
        edge_value_min: Edge,
    ) {
        debug_assert!(self
            .edges
            .get(edge_min)
            .filter(|&&ConnectionIndexes { connected_to, .. }| connected_to == vertices.max().key())
            .is_none());

        let vertex_min = vertices.min().key();

        let vertex_max = vertices.max().key();

        let edge_value = self.store_edge_value(edge_value_min);

        self.edges.insert(
            edge_min,
            ConnectionIndexes {
                connected_to: vertex_max,
                edge_value,
            },
        );

        self.edges_ranges[vertex_min].end += 1;

        self.edges_ranges[(vertex_min + 1)..=vertex_max]
            .iter_mut()
            .for_each(|edges_range| {
                edges_range.start += 1;

                edges_range.end += 1;
            });
    }

    /// Creates edge and links the maximum vertex to the minimum vertex, while
    /// also adjusting all indexes after the maximum vertex.
    ///
    /// # Panics
    /// * When the maximum vertex edge to the minimum vertex already exists.
    ///
    /// # Safety
    /// * This method adjusts indexes up to the maximum vertex, included. It
    ///   needs to be called ***after*** [`Self::create_edge_from_min_unchecked`].
    fn create_edge_from_max_unchecked(
        &mut self,
        vertices: ComparedPairNotEq<usize, ()>,
        edge_value_max: Edge,
    ) {
        let vertex_min = vertices.min().key();

        let vertex_max = vertices.max().key();

        let edge_value = self.store_edge_value(edge_value_max);

        let edges_range = &mut self.edges_ranges[vertex_max];

        let Some(edge) = self.edges[edges_range.clone()]
            .binary_search_by(|&ConnectionIndexes { connected_to, .. }| {
                connected_to.cmp(&vertex_min)
            })
            .err()
            .map(|edge| edges_range.start + edge)
        else {
            unreachable!("Edge from maximum vertex to minimum vertex should not exist!");
        };

        self.edges.insert(
            edge,
            ConnectionIndexes {
                connected_to: vertex_min,
                edge_value,
            },
        );

        edges_range.end += 1;

        self.edges_ranges[vertex_max + 1..]
            .iter_mut()
            .for_each(|Range { start, end }| {
                *start += 2;

                *end += 2;
            });
    }

    fn store_edge_value(&mut self, edge: Edge) -> usize {
        let index = self.edge_values.len();

        self.edge_values.push(edge);

        index
    }
}
