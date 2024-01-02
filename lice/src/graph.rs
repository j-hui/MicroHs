use crate::comb::{Expr, Index, Program};
use petgraph::{
    stable_graph::{DefaultIx, NodeIndex, StableGraph},
    visit::Dfs,
    Directed,
    Direction::Outgoing,
};

#[derive(Debug, Clone)]
pub struct CombNode<T> {
    pub expr: Expr,
    pub reachable: bool,
    pub redex: bool,
    pub meta: T,
}

impl<T> CombNode<T> {
    pub fn map<U>(&self, f: impl FnOnce(&T) -> U) -> CombNode<U> {
        CombNode {
            expr: self.expr.clone(),
            reachable: self.reachable,
            redex: self.redex,
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

impl<T> CombGraph<T> {
    /// Mark all unreachable nodes as such.
    ///
    /// First marks everything unreachable, then marks reachable nodes through DFS.
    pub fn mark(&mut self) {
        // TODO: is there really no better way to do this?
        let nxs = self.g.node_indices().collect::<Vec<NodeIndex>>();
        for nx in nxs {
            self.g.node_weight_mut(nx).unwrap().reachable = false;
        }

        let mut dfs = Dfs::new(&self.g, self.root);
        while let Some(nx) = dfs.next(&self.g) {
            self.g[nx].reachable = true;
        }
    }

    /// Ye goode olde mark und sweepe.
    ///
    /// Note that this won't actually free up any memory, since this is a `StableGraph`.
    /// But we will assume this is fine since this data structure is primarily meant for
    /// (memory-hungry) compile-time analysis anyway.
    pub fn gc(&mut self) {
        self.mark();
        self.g.retain_nodes(|g, nx| g[nx].reachable);
    }

    pub fn print_leaves(&self) {
        for nx in self.g.externals(Outgoing) {
            println!("{}", self.g[nx].expr);
        }
    }
}

impl From<&Program> for CombGraph<Index> {
    fn from(program: &Program) -> Self {
        let mut index = Vec::new();
        index.resize(program.body.len(), None);

        let mut g = StableGraph::new();
        for (i, expr) in program.body.iter().enumerate() {
            let this = if let Some(this) = index[i] {
                *g.node_weight_mut(this).unwrap() = Some((expr, i));
                this
            } else {
                let this = g.add_node(Some((expr, i)));
                index[i] = Some(this);
                this
            };

            match &expr {
                Expr::App(f, _, a) => {
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
                Expr::Ref(r) => {
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
                |_, expr| {
                    let (expr, i) = expr.unwrap();
                    CombNode {
                        expr: expr.clone(),
                        meta: i,
                        reachable: true, // by construction
                        redex: false,    // assume irreducible at first
                    }
                },
                |_, &e| e,
            ),
            root: index[program.root].unwrap(),
        }
    }
}
