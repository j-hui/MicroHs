use std::{cell::Cell, mem};

use crate::comb::{Combinator, Expr, Index, Prim, Program};
use petgraph::{
    stable_graph::{DefaultIx, NodeIndex, StableGraph},
    visit::{Dfs, EdgeRef, VisitMap, Visitable},
    Directed,
    Direction::{Incoming, Outgoing},
};

#[derive(Debug, Clone)]
pub struct CombNode<T> {
    pub expr: Expr,
    pub reachable: Cell<bool>,
    pub redex: Cell<Option<Combinator>>,
    pub meta: T,
}

impl<T> CombNode<T> {
    pub fn map<U>(&self, f: impl FnOnce(&T) -> U) -> CombNode<U> {
        CombNode {
            expr: self.expr.clone(),
            reachable: self.reachable.clone(),
            redex: self.redex.clone(),
            meta: f(&self.meta),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CombEdge {
    Fun,
    Arg,
    Ind,
    Arr,
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
        for n in self.g.node_weights_mut() {
            n.reachable.set(false);
        }

        let mut dfs = Dfs::new(&self.g, self.root);
        while let Some(nx) = dfs.next(&self.g) {
            self.g[nx].reachable.set(true);
        }
    }

    /// Ye goode olde mark und sweepe.
    ///
    /// Note that this won't actually free up any memory, since this is a `StableGraph`.
    /// But we will assume this is fine since this data structure is primarily meant for
    /// (memory-hungry) compile-time analysis anyway.
    pub fn gc(&mut self) {
        self.mark();
        self.g.retain_nodes(|g, nx| g[nx].reachable.get());
    }

    pub fn mark_redexes(&mut self) {
        let mut visited = self.g.visit_map();

        for nx in self.g.externals(Outgoing) {
            let Expr::Prim(Prim::Combinator(comb)) = &self.g[nx].expr else {
                continue;
            };

            let mut more = vec![nx];

            for _ in 0..comb.arity() {
                let nodes: Vec<NodeIndex> = mem::take(&mut more);
                for nx in nodes {
                    if !visited.visit(nx) {
                        continue;
                    }
                    let mut in_edges: Vec<_> = self.g.edges_directed(nx, Incoming).collect();
                    while let Some(e) = in_edges.pop() {
                        match e.weight() {
                            CombEdge::Fun => more.push(e.source()),
                            CombEdge::Ind => {
                                let ix = e.source();
                                assert!(matches!(self.g[ix].expr, Expr::Ref(_)));
                                in_edges.extend(self.g.edges_directed(ix, Incoming));
                            }
                            _ => continue,
                        }
                    }
                }
            }

            for nx in more {
                // No need to check visited here, already monotonic
                println!("Found redex for {}", comb);
                self.g[nx].redex.set(Some(*comb));
            }
        }
    }

    pub fn print_leaves(&self) {
        for nx in self.g.externals(Outgoing) {
            println!(
                "{}: reachable={}",
                self.g[nx].expr,
                self.g[nx].reachable.get()
            );
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
                Expr::Array(_, arr) => {
                    for a in arr {
                        let e = program.defs[*a];
                        let e = index[e].unwrap_or_else(|| {
                            let that = g.add_node(None);
                            index[e] = Some(that);
                            that
                        });
                        g.add_edge(this, e, CombEdge::Arr);
                    }
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
                        reachable: Cell::new(true), // reachable by construction
                        redex: Cell::new(None),    // assume irreducible at first
                        meta: i,
                    }
                },
                |_, &e| e,
            ),
            root: index[program.root].unwrap(),
        }
    }
}
