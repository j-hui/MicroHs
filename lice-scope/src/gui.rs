//! All the stuff related to graph GUI.
use egui::{
    epaint::{CubicBezierShape, TextShape},
    Color32, FontFamily, FontId, Pos2, Shape, Stroke, Vec2,
};
use egui_graphs::{
    default_edge_transform, to_graph_custom, DefaultEdgeShape, DefaultNodeShape, DisplayEdge,
    DisplayNode, DrawContext, EdgeProps, Graph, Node, NodeProps,
};
use lice::{
    comb::{Expr, Index, Program},
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
    let g = CombGraph::from(p);
    g.print_leaves();
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
pub struct NodeShape(DefaultNodeShape);

#[derive(Debug, Clone)]
pub struct EdgeShape(DefaultEdgeShape, Color32);

impl From<NodeProps<GuiNode>> for NodeShape {
    fn from(props: NodeProps<GuiNode>) -> Self {
        Self(DefaultNodeShape {
            pos: props.location,
            selected: props.selected,
            dragged: props.dragged,
            label_text: props.label,
            radius: 16.0,
        })
    }
}

impl From<EdgeProps<GuiEdge>> for EdgeShape {
    fn from(props: EdgeProps<GuiEdge>) -> Self {
        let s = DefaultEdgeShape {
            order: props.order,
            selected: props.selected,

            width: 1.,
            tip_size: 5.,
            tip_angle: std::f32::consts::TAU / 15.,

            // Only relevant if order is non-zero
            curve_size: 20.,
            loop_size: 3.,
        };
        Self(
            s,
            match props.payload {
                GuiEdge::Fun => Color32::RED,
                GuiEdge::Arg => Color32::DARK_RED,
                GuiEdge::Ind => Color32::BLUE,
            },
        )
    }
}

impl DisplayNode<GuiNode, GuiEdge, CombTy, CombIx> for NodeShape {
    fn shapes(&mut self, ctx: &DrawContext) -> Vec<egui::Shape> {
        let style = match self.0.selected || self.0.dragged {
            true => ctx.ctx.style().visuals.widgets.active,
            false => ctx.ctx.style().visuals.widgets.inactive,
        };
        let color = style.fg_stroke.color;
        let center = ctx.meta.canvas_to_screen_pos(self.0.pos);
        let size = ctx.meta.canvas_to_screen_size(self.0.radius);
        let galley = ctx.ctx.fonts(|f| {
            f.layout_no_wrap(
                self.0.label_text.clone(),
                FontId::new(size, FontFamily::Monospace),
                color,
            )
        });
        let label_shape = TextShape::new(center - galley.size() / 2., galley);
        vec![label_shape.into()]
    }

    fn update(&mut self, state: &NodeProps<GuiNode>) {
        <DefaultNodeShape as DisplayNode<GuiNode, GuiEdge, CombTy, CombIx>>::update(
            &mut self.0,
            state,
        );
    }

    fn closest_boundary_point(&self, dir: egui::Vec2) -> egui::Pos2 {
        <DefaultNodeShape as DisplayNode<GuiNode, GuiEdge, CombTy, CombIx>>::closest_boundary_point(
            &self.0, dir,
        )
    }

    fn is_inside(&self, pos: egui::Pos2) -> bool {
        <DefaultNodeShape as DisplayNode<GuiNode, GuiEdge, CombTy, CombIx>>::is_inside(&self.0, pos)
    }
}

impl DisplayEdge<GuiNode, GuiEdge, CombTy, CombIx, NodeShape> for EdgeShape {
    fn shapes(
        &mut self,
        start: &Node<GuiNode, GuiEdge, CombTy, CombIx, NodeShape>,
        end: &Node<GuiNode, GuiEdge, CombTy, CombIx, NodeShape>,
        ctx: &DrawContext,
    ) -> Vec<egui::Shape> {
        let color = match self.0.selected {
            true => ctx.ctx.style().visuals.widgets.active.fg_stroke.color,
            false => self.1,
        };

        if start.id() == end.id() {
            // draw loop
            let node_size = {
                let left_dir = Vec2::new(-1., 0.);
                let connector_left = start.display().closest_boundary_point(left_dir);
                let connector_right = start.display().closest_boundary_point(-left_dir);

                (connector_right.x - connector_left.x) / 2.
            };
            let stroke = Stroke::new(self.0.width * ctx.meta.zoom, color);
            return vec![shape_looped(
                ctx.meta.canvas_to_screen_size(node_size),
                ctx.meta.canvas_to_screen_pos(start.location()),
                stroke,
                &self.0,
            )
            .into()];
        }

        let dir = (end.location() - start.location()).normalized();
        let start_connector_point = start.display().closest_boundary_point(dir);
        let end_connector_point = end.display().closest_boundary_point(-dir);

        let tip_end = end_connector_point;

        let edge_start = start_connector_point;
        let edge_end = end_connector_point - self.0.tip_size * dir;

        let stroke_edge = Stroke::new(self.0.width * ctx.meta.zoom, color);
        let stroke_tip = Stroke::new(0., color);

        let line = Shape::line_segment(
            [
                ctx.meta.canvas_to_screen_pos(edge_start),
                ctx.meta.canvas_to_screen_pos(edge_end),
            ],
            stroke_edge,
        );

        let tip_start_1 = tip_end - self.0.tip_size * rotate_vector(dir, self.0.tip_angle);
        let tip_start_2 = tip_end - self.0.tip_size * rotate_vector(dir, -self.0.tip_angle);

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
        <DefaultEdgeShape as DisplayEdge<GuiNode, GuiEdge, CombTy, CombIx, NodeShape>>::update(
            &mut self.0,
            state,
        );
    }

    fn is_inside(
        &self,
        start: &Node<GuiNode, GuiEdge, CombTy, CombIx, NodeShape>,
        end: &Node<GuiNode, GuiEdge, CombTy, CombIx, NodeShape>,
        pos: egui::Pos2,
    ) -> bool {
        <DefaultEdgeShape as DisplayEdge<GuiNode, GuiEdge, CombTy, CombIx, NodeShape>>::is_inside(
            &self.0, start, end, pos,
        )
    }
}

fn shape_looped(
    node_size: f32,
    node_center: Pos2,
    stroke: Stroke,
    e: &DefaultEdgeShape,
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

    let loop_size = node_size * (e.loop_size + e.order as f32);

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
