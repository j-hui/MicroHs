use std::{fs::File, io::Read, path::PathBuf, str::FromStr};

use eframe::{run_native, App, CreationContext};
use egui::{Context, Pos2};
use egui_graphs::{
    default_edge_transform, to_graph_custom, GraphView, Node, SettingsInteraction,
    SettingsNavigation, SettingsStyle,
};
use lice::comb::CombFile;

use crate::gui::{CombEdgeShape, CombNodeShape, GuiCombGraph};

pub struct CombApp {
    g: GuiCombGraph,
    filename: PathBuf,
}

impl CombApp {
    pub fn new(_: &CreationContext<'_>, filename: PathBuf) -> Self {
        let mut buf = String::new();
        let mut f = File::open(&filename).unwrap();
        f.read_to_string(&mut buf).unwrap();
        let c = CombFile::from_str(&buf).unwrap();

        let g = to_graph_custom(
            &c.program.to_graph(),
            |ni, n| {
                let mut node = Node::new(n.clone());
                node.set_label(n.cell.to_string());
                node.bind(
                    ni,
                    // NOTE: vertical pos is inverted
                    Pos2::new(n.meta.x_pos * 50.0, n.meta.depth as f32 * 50.0),
                );
                node
            },
            default_edge_transform,
        );

        Self { g, filename }
    }

    pub fn run(filename: PathBuf) {
        let native_options = eframe::NativeOptions::default();
        run_native(
            filename.clone().to_str().unwrap(),
            native_options,
            Box::new(|cc| Box::new(CombApp::new(cc, filename))),
        )
        .unwrap();
    }
}

impl App for CombApp {
    fn update(&mut self, ctx: &Context, _: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add(
                &mut GraphView::<_, _, _, _, CombNodeShape, CombEdgeShape>::new(&mut self.g)
                    .with_interactions(&SettingsInteraction::new().with_dragging_enabled(true))
                    .with_styles(&SettingsStyle::new().with_labels_always(true))
                    .with_navigations(
                        &SettingsNavigation::new()
                            .with_zoom_and_pan_enabled(true)
                            .with_fit_to_screen_enabled(false),
                    ),
            );
        });
    }
}
