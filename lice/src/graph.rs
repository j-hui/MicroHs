use std::{cell::Cell, mem};

use crate::comb::{Combinator, Expr, Index, Prim, Program};
use petgraph::{
    stable_graph::{self, DefaultIx, StableGraph},
    visit::{Dfs, EdgeRef, VisitMap, Visitable},
    Directed,
    Direction::{self, Incoming, Outgoing},
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

pub type NodeIndex = stable_graph::NodeIndex<CombIx>;

pub type EdgeReference<'a> = stable_graph::EdgeReference<'a, CombEdge, CombIx>;

pub struct CombGraph<T> {
    pub g: StableGraph<CombNode<T>, CombEdge, CombTy, CombIx>,

    pub root: NodeIndex,
}

impl<T: Clone> CombGraph<T> {
    /// Mark all unreachable nodes as such, and return the number of reachable nodes.
    ///
    /// First marks everything unreachable, then marks reachable nodes through DFS.
    pub fn mark(&mut self) -> usize {
        let mut size = 0;
        for n in self.g.node_weights_mut() {
            n.reachable.set(false);
        }

        let mut dfs = Dfs::new(&self.g, self.root);
        while let Some(nx) = dfs.next(&self.g) {
            self.g[nx].reachable.set(true);
            size += 1;
        }
        size
    }

    /// Ye goode olde mark und sweepe. Returns the number of sweeped nodes.
    ///
    /// Note that this won't actually free up any `petgraph` memory, since this is a `StableGraph`.
    /// But we will assume this is fine since this data structure is primarily meant for
    /// (memory-hungry) compile-time analysis anyway.
    pub fn gc(&mut self) -> usize {
        let old_size = self.g.node_count();
        self.mark();
        self.g.retain_nodes(|g, nx| g[nx].reachable.get());
        old_size - self.g.node_count()
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
                // println!("Found redex for {}", comb);
                self.g[nx].redex.set(Some(*comb));
            }
        }
    }

    pub fn follow_indirection(&self, n: NodeIndex) -> NodeIndex {
        let mut tgt = n;
        while matches!(self.g[tgt].expr, Expr::Ref(_)) {
            let mut outgoing = self.g.edges_directed(tgt, Outgoing);

            tgt = outgoing
                .next()
                .expect("Ref node should have at least one out-edge")
                .target();

            assert!(
                outgoing.next().is_none(),
                "Ref node should have exactly one out-edge"
            );
        }
        tgt
    }

    pub fn forward_indirections(&mut self) {
        self.root = self.follow_indirection(self.root);
        let mut dfs = Dfs::new(&self.g, self.root);
        while let Some(nx) = dfs.next(&self.g) {
            if let Expr::Ref(_) = self.g[nx].expr {
                // Skip indirection nodes
                continue;
            }

            let mut changes = Vec::new();

            for e in self.g.edges_directed(nx, Outgoing) {
                if !matches!(self.g[e.target()].expr, Expr::Ref(_)) {
                    continue;
                }
                let tgt = self.follow_indirection(e.target());

                changes.push((e.id(), *e.weight(), tgt));
            }

            for (e, w, tgt) in &changes {
                self.g.remove_edge(*e);

                if matches!(self.g[*tgt].expr, Expr::App(_, _, _) | Expr::Array(_, _)) {
                    // Non-leaf node; redirect pointer
                    // println!(
                    //     "Forwarding indirection to non-leaf node: {}",
                    //     self.g[*tgt].expr
                    // );
                    self.g.add_edge(nx, *tgt, *w);
                } else {
                    // Leaf node; just clone it, to keep the graph tidy
                    // println!("Forwarding indirection to leaf node: {}", self.g[*tgt].expr);
                    let tgt = self.g.add_node(self.g[*tgt].clone());
                    self.g.add_edge(nx, tgt, *w);
                }
            }
        }
    }

    pub fn collect_redex(&self, top: NodeIndex, comb: Combinator) -> Vec<NodeIndex> {
        let mut args = Vec::new();

        let mut app = top;
        for _ in 0..comb.arity() {
            assert!(
                matches!(self.g[app].expr, Expr::App(_, _, _)),
                "expected @ node in redex, instead found {}",
                self.g[app].expr
            );

            let mut outgoing = self.g.edges_directed(app, Outgoing);
            let first = outgoing
                .next()
                .expect("App node should have at least one out-edge");
            let second = outgoing
                .next()
                .expect("App node should have at least two out-edges");
            assert!(
                outgoing.next().is_none(),
                "App node should have exactly two out-edges"
            );

            match (first.weight(), second.weight()) {
                (CombEdge::Fun, CombEdge::Arg) => {
                    app = first.target();
                    args.push(second.target());
                }
                (CombEdge::Arg, CombEdge::Fun) => {
                    app = second.target();
                    args.push(first.target());
                }
                fs => {
                    panic!(
                        "App node should have a Fun out-edge and an Arg out-edge, instead found {fs:#?}"
                    );
                }
            }
        }

        let Expr::Prim(prim) = self.g[app].expr else {
            panic!(
                "expected redex chain to lead to primitive, got {} instead",
                self.g[app].expr
            );
        };
        assert_eq!(prim, Prim::Combinator(comb));
        args.push(app);
        args.reverse();
        args
    }

    pub fn remove_edges(&mut self, nx: NodeIndex, dir: Direction) {
        let old_edges: Vec<_> = self.g.edges_directed(nx, dir).map(|e| e.id()).collect();
        for e in old_edges {
            self.g.remove_edge(e);
        }
    }

    pub fn set_app(&mut self, nx: NodeIndex, f: NodeIndex, a: NodeIndex) {
        self.remove_edges(nx, Outgoing);
        self.g[nx].expr = Expr::new_app(); // TODO: add valid indices if possible?
        self.g.add_edge(nx, f, CombEdge::Fun);
        self.g.add_edge(nx, a, CombEdge::Arg);
    }

    pub fn set_ind(&mut self, nx: NodeIndex, tgt: NodeIndex) {
        self.remove_edges(nx, Outgoing);
        self.g[nx].expr = Expr::new_ref(); // TODO: add valid indices if possible?
        self.g.add_edge(nx, tgt, CombEdge::Ind);
    }

    pub fn reduce_trivial(&mut self) {
        let mut dfs = Dfs::new(&self.g, self.root);
        while let Some(nx) = dfs.next(&self.g) {
            let Some(comb) = self.g[nx].redex.get() else {
                continue;
            };
            let args = self.collect_redex(nx, comb);
            match comb {
                Combinator::I => self.set_ind(nx, args[1]),
                Combinator::A => self.set_ind(nx, args[2]),
                Combinator::K => self.set_ind(nx, args[1]),
                Combinator::K2 => self.set_ind(nx, args[1]),
                Combinator::K3 => self.set_ind(nx, args[1]),
                Combinator::K4 => self.set_ind(nx, args[1]),
                Combinator::Z => self.set_app(nx, args[1], args[2]),
                Combinator::U => self.set_app(nx, args[2], args[1]),
                Combinator::Y => self.set_app(nx, args[1], nx),
                _ => continue, // Not going to bother
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
                        redex: Cell::new(None),     // assume irreducible at first
                        meta: i,
                    }
                },
                |_, &e| e,
            ),
            root: index[program.root].unwrap(),
        }
    }
}
