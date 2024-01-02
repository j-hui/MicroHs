use crate::comb::{Cell, Index, Program};
use petgraph::{
    stable_graph::{DefaultIx, NodeIndex, StableGraph},
    Directed,
};

#[derive(Debug, Clone)]
pub struct CombNode<T> {
    pub cell: Cell,
    pub reachable: bool,
    pub meta: T,
}

impl<T> CombNode<T> {
    pub fn map<U>(&self, f: impl FnOnce(&T) -> U) -> CombNode<U> {
        CombNode {
            cell: self.cell.clone(),
            reachable: self.reachable,
            meta: f(&self.meta),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CombEdge {
    Fun,
    Arg,
    Ind,
}

pub type CombIx = DefaultIx;

pub type CombTy = Directed;

pub struct CombGraph<T> {
    pub g: StableGraph<CombNode<T>, CombEdge, CombTy, CombIx>,

    pub root: NodeIndex,
}

impl From<&Program> for CombGraph<Index> {
    fn from(program: &Program) -> Self {
        let mut index = Vec::new();
        index.resize(program.body.len(), None);

        let mut g = StableGraph::new();
        for (i, cell) in program.body.iter().enumerate() {
            let this = if let Some(this) = index[i] {
                *g.node_weight_mut(this).unwrap() = Some((cell, i));
                this
            } else {
                let this = g.add_node(Some((cell, i)));
                index[i] = Some(this);
                this
            };

            match &cell {
                Cell::App(f, _, a) => {
                    let f = index[*f].unwrap_or_else(|| {
                        let that = g.add_node(None);
                        index[*f] = Some(that);
                        that
                    });
                    g.add_edge(this, f, CombEdge::Fun);

                    let a = index[*a].unwrap_or_else(|| {
                        let that = g.add_node(None);
                        index[*a] = Some(that);
                        that
                    });
                    g.add_edge(this, a, CombEdge::Arg);
                }
                Cell::Ref(r) => {
                    let d = program.defs[*r];

                    let d = index[d].unwrap_or_else(|| {
                        let that = g.add_node(None);
                        index[d] = Some(that);
                        that
                    });
                    g.add_edge(this, d, CombEdge::Ind);
                }
                _ => (),
            }
        }

        CombGraph {
            g: g.map(
                |_, cell| {
                    let (cell, i) = cell.unwrap();
                    CombNode {
                        cell: cell.clone(),
                        reachable: true,
                        meta: i,
                    }
                },
                |_, &e| e,
            ),
            root: index[program.root].unwrap(),
        }
    }
}
