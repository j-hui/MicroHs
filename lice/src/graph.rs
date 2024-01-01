use crate::comb::{Cell, Index, Program};
use petgraph::{
    stable_graph::{DefaultIx, StableGraph},
    Directed,
};

#[derive(Debug, Clone)]
pub struct CombNode {
    pub cell: Cell,
    pub meta: CellMetadata,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CombEdge {
    Fun,
    Arg,
    Ind,
}

/// Cell metadata, acquired while parsing the combinator file.
///
/// The data in here is strictly optional, and can always be discarded when serializing cells.
#[derive(Debug, Clone, Default)]
pub struct CellMetadata {
    /// Distance to a leaf
    pub height: usize,
    /// Distance to root
    pub depth: usize,
    /// Horizontal position
    pub x_pos: f32,
    /// Whether the node is reachable
    pub reachable: bool,
}

pub type CombIx = DefaultIx;

pub type CombTy = Directed;

pub type CombGraph = StableGraph<CombNode, CombEdge, CombTy, CombIx>;

impl Program {
    fn build_metadata(&self) -> Vec<CellMetadata> {
        let mut metadata = Vec::new();
        metadata.resize_with(self.body.len(), Default::default);
        let mut x_pos = 1.0;
        self.build_meta(&mut metadata, &mut x_pos, self.root, 0);
        metadata
    }

    fn build_meta(
        &self,
        metadata: &mut Vec<CellMetadata>,
        x_pos: &mut f32,
        i: Index,
        depth: usize,
    ) {
        metadata[i].depth = depth;
        metadata[i].reachable = true;

        match &self.body[i] {
            Cell::App(f, _, a) => {
                let (f, a) = (*f, *a);
                self.build_meta(metadata, x_pos, f, depth + 1);
                self.build_meta(metadata, x_pos, a, depth + 1);

                metadata[i].height = usize::max(metadata[f].height, metadata[a].height);
                metadata[i].x_pos = (metadata[f].x_pos + metadata[a].x_pos) / 2.0;
            }
            Cell::Array(_, arr) => {
                let mut h = 0;
                let mut x = 0.0;
                for &a in arr {
                    self.build_meta(metadata, x_pos, a, depth + 1);
                    h = h.max(metadata[i].height);
                    x += metadata[i].x_pos;
                }
                metadata[i].height = h;
                metadata[i].x_pos = x / arr.len() as f32;
            }
            _ => {
                metadata[i].height = 0;
                metadata[i].x_pos = *x_pos;
                *x_pos += 1.0;
            }
        }
    }

    pub fn to_graph(&self) -> CombGraph {
        let metadata = self.build_metadata();

        let mut index_to_node = Vec::new();
        index_to_node.resize(self.body.len(), None);

        let mut g = StableGraph::new();
        for (i, cell) in self.body.iter().enumerate() {
            let this = if let Some(this) = index_to_node[i] {
                *g.node_weight_mut(this).unwrap() = Some((cell, &metadata[i]));
                this
            } else {
                let this = g.add_node(Some((cell, &metadata[i])));
                index_to_node[i] = Some(this);
                this
            };

            match &cell {
                Cell::App(f, _, a) => {
                    let f = index_to_node[*f].unwrap_or_else(|| {
                        let that = g.add_node(None);
                        index_to_node[*f] = Some(that);
                        that
                    });
                    g.add_edge(this, f, CombEdge::Fun);

                    let a = index_to_node[*a].unwrap_or_else(|| {
                        let that = g.add_node(None);
                        index_to_node[*a] = Some(that);
                        that
                    });
                    g.add_edge(this, a, CombEdge::Arg);
                }
                Cell::Ref(r) => {
                    let d = self.defs[*r];

                    let d = index_to_node[d].unwrap_or_else(|| {
                        let that = g.add_node(None);
                        index_to_node[d] = Some(that);
                        that
                    });
                    g.add_edge(this, d, CombEdge::Ind);
                }
                _ => (),
            }
        }

        g.map(
            |_, o| {
                let (cell, meta) = o.unwrap();
                CombNode {
                    cell: cell.clone(),
                    meta: meta.clone(),
                }
            },
            |_, &e| e,
        )
    }
}
