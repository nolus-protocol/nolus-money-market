use std::borrow::Borrow;

use super::{ConnectionIndexes, Graph};

pub struct ConnectedVertices<'r, Vertex, Edge> {
    graph: &'r Graph<Vertex, Edge>,
    cut_off: Option<usize>,
    vertex: usize,
    connected_vertices: &'r [ConnectionIndexes],
}

impl<'r, Vertex, Edge> ConnectedVertices<'r, Vertex, Edge> {
    pub(super) fn new(graph: &'r Graph<Vertex, Edge>, vertex: usize) -> Self {
        Self {
            graph,
            cut_off: None,
            vertex,
            connected_vertices: &graph.edges[graph.edges_ranges[vertex].clone()],
        }
    }
}

impl<'r, Vertex, Edge> ConnectedVertices<'r, Vertex, Edge> {
    pub fn vertex(&self) -> &Vertex {
        &self.graph.vertices[self.vertex]
    }

    pub fn get_edge<'t, 'u, T>(&'t self, vertex: &'u T) -> Option<&Edge>
    where
        Vertex: Borrow<T>,
        T: Ord + ?Sized,
    {
        self.graph
            .get_edge(self.graph.vertices[self.vertex].borrow(), vertex)
    }

    pub fn iter_neighbors_without_backlink(
        &self,
    ) -> impl DoubleEndedIterator<Item = ConnectedVertices<'_, Vertex, Edge>> + '_ {
        self.iter_neighbors(Some(self.vertex))
    }

    pub fn iter_neighbors_with_backlink(
        &self,
    ) -> impl DoubleEndedIterator<Item = ConnectedVertices<'_, Vertex, Edge>> + '_ {
        self.iter_neighbors(self.cut_off)
    }

    #[inline]
    pub fn walk_to_without_backlink<'t, 'u, T>(
        &'t self,
        neighbor: &'u T,
    ) -> Option<ConnectedVertices<'t, Vertex, Edge>>
    where
        Vertex: Borrow<T>,
        T: Ord + ?Sized,
    {
        self.walk_to(neighbor, Some(self.vertex))
    }

    #[inline]
    pub fn walk_to_with_backlink<'t, 'u, T>(
        &'t self,
        neighbor: &'u T,
    ) -> Option<ConnectedVertices<'t, Vertex, Edge>>
    where
        Vertex: Borrow<T>,
        T: Ord + ?Sized,
    {
        self.walk_to(neighbor, self.cut_off)
    }

    pub fn iter_neighbors(
        &self,
        cut_off: Option<usize>,
    ) -> impl DoubleEndedIterator<Item = ConnectedVertices<'_, Vertex, Edge>> + '_ {
        self.connected_vertices
            .iter()
            .map(|&ConnectionIndexes { connected_to, .. }| connected_to)
            .filter(move |&connected_to| {
                self.cut_off.map_or(true, |cut_off| connected_to != cut_off)
            })
            .map(move |connected_to| ConnectedVertices {
                graph: self.graph,
                cut_off,
                vertex: connected_to,
                connected_vertices: &self.graph.edges
                    [self.graph.edges_ranges[connected_to].clone()],
            })
    }

    fn walk_to<'t, 'u, T>(
        &'t self,
        neighbor: &'u T,
        cut_off: Option<usize>,
    ) -> Option<ConnectedVertices<'t, Vertex, Edge>>
    where
        Vertex: Borrow<T>,
        T: Ord + ?Sized,
    {
        self.connected_vertices
            .iter()
            .find_map(|&ConnectionIndexes { connected_to, .. }| {
                (self.graph.vertices[connected_to].borrow() == neighbor).then_some(connected_to)
            })
            .filter(move |&vertex| self.cut_off.map_or(true, |cut_off| vertex != cut_off))
            .map(|vertex| ConnectedVertices {
                graph: self.graph,
                cut_off,
                vertex,
                connected_vertices: &self.graph.edges[self.graph.edges_ranges[vertex].clone()],
            })
    }
}

#[test]
fn test_connected_vertices_self() {
    const START_VERTEX: &str = "v8";

    let graph = super::well_known_graph();

    let connected_vertices = graph.connected_vertices(START_VERTEX).unwrap();

    assert_eq!(*connected_vertices.vertex(), START_VERTEX);
}

#[test]
fn test_first_layer_walk() {
    const START_VERTEX: &str = "v8";
    const FIRST_LAYER: &[&str] = &["v7", "v9"];

    let graph = super::well_known_graph();

    let connected_vertices = graph.connected_vertices(START_VERTEX).unwrap();

    assert_eq!(*connected_vertices.vertex(), START_VERTEX);

    let first_layer: Vec<_> = connected_vertices
        .iter_neighbors_with_backlink()
        .map(|connected_vertices| *connected_vertices.vertex())
        .collect();

    assert_eq!(
        first_layer,
        connected_vertices
            .iter_neighbors_without_backlink()
            .map(|connected_vertices| *connected_vertices.vertex())
            .collect::<Vec<_>>()
    );

    assert_eq!(first_layer, FIRST_LAYER);
}

#[test]
fn test_second_layer_walk_without_backlink() {
    const START_VERTEX: &str = "v8";
    const SECOND_LAYER: &[&[&str]] = &[&["v5"], &["v10", "v11"]];

    let graph = super::well_known_graph();

    let connected_vertices = graph.connected_vertices(START_VERTEX).unwrap();

    assert_eq!(
        connected_vertices
            .iter_neighbors_without_backlink()
            .map(|connected_vertices| {
                connected_vertices
                    .iter_neighbors_without_backlink()
                    .map(|connected_vertices| *connected_vertices.vertex())
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>(),
        SECOND_LAYER
    );
}

#[test]
fn test_second_layer_walk_with_backlink() {
    const START_VERTEX: &str = "v8";
    const SECOND_LAYER_AND_START_VERTEX: &[&[&str]] = &[&["v5", "v8"], &["v10", "v11", "v8"]];

    let graph = super::well_known_graph();

    let connected_vertices = graph.connected_vertices(START_VERTEX).unwrap();

    assert_eq!(
        connected_vertices
            .iter_neighbors_with_backlink()
            .map(|connected_vertices| {
                connected_vertices
                    .iter_neighbors_with_backlink()
                    .map(|connected_vertices| *connected_vertices.vertex())
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>(),
        SECOND_LAYER_AND_START_VERTEX
    );
}
