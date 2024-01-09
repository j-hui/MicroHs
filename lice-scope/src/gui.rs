//! All the stuff related to graph GUI.
use egui::{
    epaint::{CubicBezierShape, TextShape},
    Color32, FontFamily, FontId, Pos2, Shape, Stroke, Vec2,
};
use egui_graphs::{
    default_edge_transform, to_graph_custom, DisplayEdge, DisplayNode, DrawContext, EdgeProps,
    Graph, Node, NodeProps,
};
use lice::{
    comb::{Expr, Index, Program, Combinator},
    graph::{CombEdge, CombGraph, CombIx, CombNode, CombTy},
};

pub type GuiNode = CombNode<NodeMetadata>;
pub type GuiEdge = CombEdge;

/// GUI for [`lice::graph::CombGraph`].
pub type GuiGraph = Graph<GuiNode, GuiEdge, CombTy, CombIx, NodeShape, EdgeShape>;

/// Cell metadata, acquired while parsing the combinator file.
///
/// The data in here is strictly optional, and can always be discarded when serializing cells.
#[derive(Debug, Clone, Default)]
pub struct NodeMetadata {
    /// Distance to a leaf
    pub height: usize,
    /// Distance to root
    pub depth: usize,
    /// Horizontal position
    pub x_pos: f32,
}

fn build_metadata(p: &Program) -> Vec<NodeMetadata> {
    let mut metadata = Vec::new();
    metadata.resize_with(p.body.len(), Default::default);
    let mut x_pos = 1.0;
    build_meta(p, &mut metadata, &mut x_pos, p.root, 0);
    metadata
}

fn build_meta(
    p: &Program,
    metadata: &mut Vec<NodeMetadata>,
    x_pos: &mut f32,
    i: Index,
    depth: usize,
) {
    metadata[i].depth = depth;

    match &p.body[i] {
        Expr::App(f, _, a) => {
            let (f, a) = (*f, *a);
            build_meta(p, metadata, x_pos, f, depth + 1);
            build_meta(p, metadata, x_pos, a, depth + 1);

            metadata[i].height = usize::max(metadata[f].height, metadata[a].height);
            metadata[i].x_pos = (metadata[f].x_pos + metadata[a].x_pos) / 2.0;
        }
        Expr::Array(_, arr) => {
            let mut h = 0;
            let mut x = 0.0;
            for &a in arr {
                build_meta(p, metadata, x_pos, a, depth + 1);
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

pub fn to_gui_graph(p: &Program) -> GuiGraph {
    let metadata = build_metadata(p);
    let mut g = CombGraph::from(p);

    // Begin: an ad hoc set of transformations that should probably be done interactively/elsewhere
    println!("Before: {} nodes", g.g.node_count());
    g.forward_indirections();
    g.mark_redexes();
    g.reduce_trivial();
    g.forward_indirections();
    g.mark();
    g.gc();
    g.mark_redexes();
    println!("After: {} nodes", g.g.node_count());
    // End: an ad hoc set of transformations that should probably be done interactively/elsewhere

    to_graph_custom(
        &g.g.map(|_, n| n.map(|&i| metadata[i].clone()), |_, &e| e),
        |ni, n| {
            let mut node = Node::new(n.clone());
            node.set_label(n.expr.to_string());
            node.bind(
                ni,
                // NOTE: vertical pos is inverted
                Pos2::new(
                    node.payload().meta.x_pos * 50.0,
                    node.payload().meta.depth as f32 * 50.0,
                ),
            );
            node
        },
        default_edge_transform,
    )
}

#[derive(Debug, Clone)]
pub struct NodeShape {
    center: Pos2,
    selected: bool,
    dragged: bool,
    radius: f32,
    label_text: String,
    reachable: bool,
    redex: Option<Combinator>,
}

#[derive(Debug, Clone)]
pub struct EdgeShape {
    kind: CombEdge,
    width: f32,
    tip_size: f32,
    tip_angle: f32,
}

impl NodeShape {
    const RADIUS: f32 = 16.0;
}

impl From<NodeProps<GuiNode>> for NodeShape {
    fn from(props: NodeProps<GuiNode>) -> Self {
        Self {
            center: props.location,
            selected: props.selected,
            dragged: props.dragged,
            radius: Self::RADIUS,
            label_text: props.label,
            reachable: props.payload.reachable.get(),
            redex: props.payload.redex.get(),
        }
    }
}

impl EdgeShape {
    const WIDTH: f32 = 1.0;
    const TIP_SIZE: f32 = 5.0;
    const TIP_ANGLE: f32 = std::f32::consts::TAU / 15.0;

    fn color(&self) -> Color32 {
        match self.kind {
            CombEdge::Fun => Color32::RED,
            CombEdge::Arg => Color32::DARK_RED,
            CombEdge::Ind => Color32::BLUE,
            CombEdge::Arr => Color32::GREEN,
        }
    }
}

impl From<EdgeProps<GuiEdge>> for EdgeShape {
    fn from(props: EdgeProps<GuiEdge>) -> Self {
        Self {
            width: Self::WIDTH,
            tip_size: Self::TIP_SIZE,
            tip_angle: Self::TIP_ANGLE,
            kind: props.payload,
        }
    }
}

impl DisplayNode<GuiNode, GuiEdge, CombTy, CombIx> for NodeShape {
    fn shapes(&mut self, ctx: &DrawContext) -> Vec<egui::Shape> {
        let color = match (self.selected, self.dragged, self.reachable) {
            (_, _, false) => ctx.ctx.style().visuals.widgets.active.bg_stroke.color,
            (true, _, _) | (_, true, _) => ctx.ctx.style().visuals.widgets.active.text_color(),
            _ => ctx.ctx.style().visuals.widgets.inactive.text_color(),
        };
        let center = ctx.meta.canvas_to_screen_pos(self.center);
        let size = ctx.meta.canvas_to_screen_size(self.radius);
        let galley = ctx.ctx.fonts(|f| {
            f.layout_no_wrap(
                if self.redex.is_some() {
                    format!("[{}]", self.label_text)
                } else {
                    self.label_text.clone()
                },
                FontId::new(size, FontFamily::Monospace),
                color,
            )
        });
        let label_shape = TextShape::new(center - galley.size() / 2., galley);
        vec![label_shape.into()]
    }

    fn update(&mut self, state: &NodeProps<GuiNode>) {
        self.center = state.location;
        self.selected = state.selected;
        self.dragged = state.dragged;
        self.label_text = state.label.to_string();
        self.reachable = state.payload.reachable.get();
        self.redex = state.payload.redex.get();
    }

    fn closest_boundary_point(&self, dir: egui::Vec2) -> egui::Pos2 {
        self.center + dir.normalized() * self.radius
    }

    fn is_inside(&self, pos: egui::Pos2) -> bool {
        let dir = pos - self.center;
        dir.length() <= self.radius
    }
}

impl DisplayEdge<GuiNode, GuiEdge, CombTy, CombIx, NodeShape> for EdgeShape {
    fn shapes(
        &mut self,
        start: &Node<GuiNode, GuiEdge, CombTy, CombIx, NodeShape>,
        end: &Node<GuiNode, GuiEdge, CombTy, CombIx, NodeShape>,
        ctx: &DrawContext,
    ) -> Vec<egui::Shape> {
        let color = self.color();

        if start.id() == end.id() {
            // draw loop
            let node_size = {
                let left_dir = Vec2::new(-1., 0.);
                let connector_left = start.display().closest_boundary_point(left_dir);
                let connector_right = start.display().closest_boundary_point(-left_dir);

                (connector_right.x - connector_left.x) / 2.
            };
            let stroke = Stroke::new(self.width * ctx.meta.zoom, color);
            return vec![shape_looped(
                ctx.meta.canvas_to_screen_size(node_size),
                ctx.meta.canvas_to_screen_pos(start.location()),
                stroke,
                3.,
            )
            .into()];
        }

        let dir = (end.location() - start.location()).normalized();
        let start_connector_point = start.display().closest_boundary_point(dir);
        let end_connector_point = end.display().closest_boundary_point(-dir);

        let tip_end = end_connector_point;

        let edge_start = start_connector_point;
        let edge_end = end_connector_point - self.tip_size * dir;

        let stroke_edge = Stroke::new(self.width * ctx.meta.zoom, color);
        let stroke_tip = Stroke::new(0., color);

        let line = Shape::line_segment(
            [
                ctx.meta.canvas_to_screen_pos(edge_start),
                ctx.meta.canvas_to_screen_pos(edge_end),
            ],
            stroke_edge,
        );

        let tip_start_1 = tip_end - self.tip_size * rotate_vector(dir, self.tip_angle);
        let tip_start_2 = tip_end - self.tip_size * rotate_vector(dir, -self.tip_angle);

        // draw tips for directed edges

        let line_tip = Shape::convex_polygon(
            vec![
                ctx.meta.canvas_to_screen_pos(tip_end),
                ctx.meta.canvas_to_screen_pos(tip_start_1),
                ctx.meta.canvas_to_screen_pos(tip_start_2),
            ],
            color,
            stroke_tip,
        );
        vec![line, line_tip]
    }

    fn update(&mut self, state: &EdgeProps<GuiEdge>) {
        // We don't need the user to interact with edges
        // self.selected = state.selected;
        self.kind = state.payload;
    }

    fn is_inside(
        &self,
        _start: &Node<GuiNode, GuiEdge, CombTy, CombIx, NodeShape>,
        _end: &Node<GuiNode, GuiEdge, CombTy, CombIx, NodeShape>,
        _pos: egui::Pos2,
    ) -> bool {
        // Kind of fussy, and not useful
        false
    }
}

fn shape_looped(
    node_size: f32,
    node_center: Pos2,
    stroke: Stroke,
    loop_size: f32,
) -> CubicBezierShape {
    let center_horizon_angle = std::f32::consts::PI / 4.;
    let y_intersect = node_center.y - node_size * center_horizon_angle.sin();

    let edge_start = Pos2::new(
        node_center.x - node_size * center_horizon_angle.cos(),
        y_intersect,
    );
    let edge_end = Pos2::new(
        node_center.x + node_size * center_horizon_angle.cos(),
        y_intersect,
    );

    let loop_size = node_size * loop_size;

    let control_point1 = Pos2::new(node_center.x + loop_size, node_center.y - loop_size);
    let control_point2 = Pos2::new(node_center.x - loop_size, node_center.y - loop_size);

    CubicBezierShape::from_points_stroke(
        [edge_end, control_point1, control_point2, edge_start],
        false,
        Color32::default(),
        stroke,
    )
}

/// rotates vector by angle
fn rotate_vector(vec: Vec2, angle: f32) -> Vec2 {
    let cos = angle.cos();
    let sin = angle.sin();
    Vec2::new(cos * vec.x - sin * vec.y, sin * vec.x + cos * vec.y)
}
