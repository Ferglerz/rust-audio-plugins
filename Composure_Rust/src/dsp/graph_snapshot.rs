//! Prepared compression graph and lookup table shared with the audio chain.

use super::compression_lut::CompressionLUT;
use super::graph::CompressionGraph;

#[derive(Debug, Clone, Copy)]
pub struct GraphSnapshot {
    pub graph: CompressionGraph,
    pub lut: CompressionLUT,
}

impl GraphSnapshot {
    pub fn from_graph(mut graph: CompressionGraph) -> Self {
        graph.ensure_segments_cached();
        let mut lut = CompressionLUT::new();
        lut.build_lut(&graph);
        Self { graph, lut }
    }
}

impl Default for GraphSnapshot {
    fn default() -> Self {
        Self::from_graph(CompressionGraph::new())
    }
}
