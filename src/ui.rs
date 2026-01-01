use crate::neighborhood::Neighborhood;
use crate::rule::{catalog, LifeLikeRule, Rule, RuleSpec};
use crate::simulation::{patterns, Pattern, Simulation};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    Pan,
    Draw,
    Erase,
    Place,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Panel {
    Rules,
    Patterns,
    Neighborhood,
    Statistics,
    Spacetime,
    Settings,
}

pub struct UiState {
    pub paused: bool,
    pub speed: u32,
    pub current_tool: Tool,
    pub active_panel: Option<Panel>,
    pub side_panel_collapsed: bool,

    pub rule_input: String,
    pub rule_parse_error: Option<String>,
    pub selected_catalog_rule: usize,

    pub birth_toggles: [bool; 9],
    pub survival_toggles: [bool; 9],

    pub selected_pattern: usize,
    pub custom_pattern_rle: String,

    pub selected_neighborhood: usize,
    pub custom_neighborhood_offsets: Vec<(i32, i32)>,
    pub neighborhood_editor_range: i32,

    pub show_grid: bool,
    pub show_stats_overlay: bool,
    pub color_scheme: ColorScheme,

    pub spacetime_slice_y: u32,
    pub spacetime_history: Vec<Vec<u8>>,
    pub spacetime_max_history: usize,

    pub spacetime_3d_history: Vec<Vec<Vec<u8>>>,
    pub spacetime_3d_max_depth: usize,
    pub spacetime_rotation_x: f32,
    pub spacetime_rotation_y: f32,
    pub spacetime_scale: f32,
    pub spacetime_view_mode: SpacetimeViewMode,

    pub seed_input: String,
    pub density_input: f32,
    pub grid_width_input: u32,
    pub grid_height_input: u32,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ColorScheme {
    Classic,
    Heat,
    Ocean,
    Forest,
    Neon,
    Grayscale,
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum SpacetimeViewMode {
    #[default]
    Slice2D,
    Isometric3D,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            paused: true,
            speed: 10,
            current_tool: Tool::Draw,
            active_panel: Some(Panel::Rules),
            side_panel_collapsed: false,

            rule_input: "B3/S23".into(),
            rule_parse_error: None,
            selected_catalog_rule: 0,

            birth_toggles: [false, false, false, true, false, false, false, false, false],
            survival_toggles: [false, false, true, true, false, false, false, false, false],

            selected_pattern: 0,
            custom_pattern_rle: String::new(),

            selected_neighborhood: 0,
            custom_neighborhood_offsets: Vec::new(),
            neighborhood_editor_range: 2,

            show_grid: false,
            show_stats_overlay: true,
            color_scheme: ColorScheme::Classic,

            spacetime_slice_y: 64,
            spacetime_history: Vec::new(),
            spacetime_max_history: 200,

            spacetime_3d_history: Vec::new(),
            spacetime_3d_max_depth: 50,
            spacetime_rotation_x: 0.5,
            spacetime_rotation_y: 0.3,
            spacetime_scale: 1.0,
            spacetime_view_mode: SpacetimeViewMode::Isometric3D,

            seed_input: "0".into(),
            density_input: 0.3,
            grid_width_input: 128,
            grid_height_input: 128,
        }
    }
}

impl UiState {
    pub fn sync_from_rule(&mut self, rule: &Rule) {
        if let RuleSpec::LifeLike(ref ll) = rule.spec {
            self.rule_input = ll.to_bs_string();
            self.birth_toggles = [false; 9];
            self.survival_toggles = [false; 9];
            for &b in &ll.birth {
                if (b as usize) < 9 {
                    self.birth_toggles[b as usize] = true;
                }
            }
            for &s in &ll.survival {
                if (s as usize) < 9 {
                    self.survival_toggles[s as usize] = true;
                }
            }
        }
    }

    pub fn build_rule_from_toggles(&self) -> Rule {
        let birth: Vec<u8> = self
            .birth_toggles
            .iter()
            .enumerate()
            .filter(|(_, &on)| on)
            .map(|(i, _)| i as u8)
            .collect();
        let survival: Vec<u8> = self
            .survival_toggles
            .iter()
            .enumerate()
            .filter(|(_, &on)| on)
            .map(|(i, _)| i as u8)
            .collect();

        let rule = LifeLikeRule::new(&birth, &survival);
        Rule {
            name: rule.to_bs_string(),
            spec: RuleSpec::LifeLike(rule.clone()),
            description: None,
            source: None,
            properties: crate::rule::RuleProperties {
                lambda: Some(rule.lambda()),
                ..Default::default()
            },
        }
    }

    pub fn try_parse_rule(&mut self) -> Option<Rule> {
        match LifeLikeRule::parse(&self.rule_input) {
            Some(rule) => {
                self.rule_parse_error = None;
                self.birth_toggles = [false; 9];
                self.survival_toggles = [false; 9];
                for &b in &rule.birth {
                    if (b as usize) < 9 {
                        self.birth_toggles[b as usize] = true;
                    }
                }
                for &s in &rule.survival {
                    if (s as usize) < 9 {
                        self.survival_toggles[s as usize] = true;
                    }
                }
                Some(Rule {
                    name: rule.to_bs_string(),
                    spec: RuleSpec::LifeLike(rule.clone()),
                    description: None,
                    source: None,
                    properties: crate::rule::RuleProperties {
                        lambda: Some(rule.lambda()),
                        ..Default::default()
                    },
                })
            }
            None => {
                self.rule_parse_error = Some("Invalid B/S notation".into());
                None
            }
        }
    }
}

pub fn draw_ui(ctx: &egui::Context, ui_state: &mut UiState, sim: &mut Simulation) {
    draw_top_bar(ctx, ui_state, sim);
    draw_side_panel(ctx, ui_state, sim);
    draw_central_panel(ctx, ui_state, sim);
    draw_stats_overlay(ctx, ui_state, sim);
}

fn draw_top_bar(ctx: &egui::Context, ui_state: &mut UiState, sim: &mut Simulation) {
    egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
        ui.horizontal(|ui| {
            if ui
                .button(if ui_state.paused {
                    "▶ Play"
                } else {
                    "⏸ Pause"
                })
                .clicked()
            {
                ui_state.paused = !ui_state.paused;
            }

            if ui.button("⏮ Step Back").clicked() && ui_state.paused {
                sim.step_back();
            }

            if ui.button("⏭ Step").clicked() && ui_state.paused {
                sim.step();
            }

            ui.separator();

            if ui.button("🔄 Reset").clicked() {
                sim.reset();
            }

            if ui.button("🎲 Randomize").clicked() {
                sim.randomize(None);
            }

            if ui.button("🗑 Clear").clicked() {
                sim.clear();
            }

            ui.separator();

            ui.label("Speed:");
            ui.add(egui::Slider::new(&mut ui_state.speed, 1..=60).show_value(false));

            ui.separator();

            ui.label(format!("Gen: {}", sim.generation()));
            ui.label(format!("Pop: {}", sim.stats().population));

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(format!("Rule: {}", sim.config.rule.canonical_id()));
            });
        });
    });
}

fn draw_side_panel(ctx: &egui::Context, ui_state: &mut UiState, sim: &mut Simulation) {
    if ui_state.side_panel_collapsed {
        egui::SidePanel::left("side_panel_collapsed")
            .exact_width(24.0)
            .resizable(false)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    if ui.button("▶").on_hover_text("Expand panel").clicked() {
                        ui_state.side_panel_collapsed = false;
                    }
                });
            });
        return;
    }

    egui::SidePanel::left("side_panel")
        .default_width(280.0)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("◀").on_hover_text("Collapse panel").clicked() {
                    ui_state.side_panel_collapsed = true;
                }
                ui.separator();
                ui.selectable_value(&mut ui_state.active_panel, Some(Panel::Rules), "Rules");
                ui.selectable_value(
                    &mut ui_state.active_panel,
                    Some(Panel::Patterns),
                    "Patterns",
                );
                ui.selectable_value(
                    &mut ui_state.active_panel,
                    Some(Panel::Neighborhood),
                    "Nbhd",
                );
                ui.selectable_value(&mut ui_state.active_panel, Some(Panel::Statistics), "Stats");
                ui.selectable_value(&mut ui_state.active_panel, Some(Panel::Spacetime), "Time");
                ui.selectable_value(&mut ui_state.active_panel, Some(Panel::Settings), "⚙");
            });

            ui.separator();

            egui::ScrollArea::vertical().show(ui, |ui| match ui_state.active_panel {
                Some(Panel::Rules) => draw_rules_panel(ui, ui_state, sim),
                Some(Panel::Patterns) => draw_patterns_panel(ui, ui_state, sim),
                Some(Panel::Neighborhood) => draw_neighborhood_panel(ui, ui_state, sim),
                Some(Panel::Statistics) => draw_statistics_panel(ui, ui_state, sim),
                Some(Panel::Spacetime) => draw_spacetime_panel(ui, ui_state, sim),
                Some(Panel::Settings) => draw_settings_panel(ui, ui_state, sim),
                None => {}
            });
        });
}

fn draw_rules_panel(ui: &mut egui::Ui, ui_state: &mut UiState, sim: &mut Simulation) {
    ui.heading("Rule Editor");

    ui.horizontal(|ui| {
        ui.label("B/S:");
        let response = ui.text_edit_singleline(&mut ui_state.rule_input);
        if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            if let Some(rule) = ui_state.try_parse_rule() {
                sim.set_rule(rule);
            }
        }
        if ui.button("Apply").clicked() {
            if let Some(rule) = ui_state.try_parse_rule() {
                sim.set_rule(rule);
            }
        }
    });

    if let Some(ref err) = ui_state.rule_parse_error {
        ui.colored_label(egui::Color32::RED, err);
    }

    ui.add_space(8.0);

    ui.label("Birth (dead → alive):");
    ui.horizontal(|ui| {
        for i in 0..9 {
            let label = format!("{}", i);
            if ui
                .selectable_label(ui_state.birth_toggles[i], &label)
                .clicked()
            {
                ui_state.birth_toggles[i] = !ui_state.birth_toggles[i];
                let rule = ui_state.build_rule_from_toggles();
                ui_state.rule_input = rule.canonical_id();
                sim.set_rule(rule);
            }
        }
    });

    ui.label("Survival (alive → alive):");
    ui.horizontal(|ui| {
        for i in 0..9 {
            let label = format!("{}", i);
            if ui
                .selectable_label(ui_state.survival_toggles[i], &label)
                .clicked()
            {
                ui_state.survival_toggles[i] = !ui_state.survival_toggles[i];
                let rule = ui_state.build_rule_from_toggles();
                ui_state.rule_input = rule.canonical_id();
                sim.set_rule(rule);
            }
        }
    });

    ui.add_space(8.0);

    if let RuleSpec::LifeLike(ref rule) = sim.config.rule.spec {
        ui.label(format!("λ = {:.3}", rule.lambda()));
    }

    ui.add_space(16.0);
    ui.separator();
    ui.heading("Rule Catalog");

    let catalog_rules = catalog::all();
    for (i, rule) in catalog_rules.iter().enumerate() {
        let selected = ui_state.selected_catalog_rule == i;
        let response = ui.selectable_label(selected, &rule.name);

        if response.clicked() {
            ui_state.selected_catalog_rule = i;
            ui_state.sync_from_rule(rule);
            sim.set_rule(rule.clone());
        }

        response.on_hover_ui(|ui| {
            if let Some(ref desc) = rule.description {
                ui.label(desc);
            }
            ui.label(format!("Rule: {}", rule.canonical_id()));
            if let Some(lambda) = rule.properties.lambda {
                ui.label(format!("λ = {:.3}", lambda));
            }
            if let Some(class) = rule.properties.wolfram_class {
                ui.label(format!("Wolfram Class: {}", class));
            }
        });
    }
}

fn draw_patterns_panel(ui: &mut egui::Ui, ui_state: &mut UiState, _sim: &mut Simulation) {
    ui.heading("Patterns");

    ui.horizontal(|ui| {
        ui.selectable_value(&mut ui_state.current_tool, Tool::Pan, "🖐 Pan");
        ui.selectable_value(&mut ui_state.current_tool, Tool::Draw, "✏ Draw");
        ui.selectable_value(&mut ui_state.current_tool, Tool::Erase, "🧹 Erase");
        ui.selectable_value(&mut ui_state.current_tool, Tool::Place, "📍 Place");
    });

    ui.add_space(8.0);
    ui.separator();
    ui.label("Pattern Library:");

    let all_patterns = patterns::all();
    for (i, pattern) in all_patterns.iter().enumerate() {
        let selected = ui_state.selected_pattern == i;
        if ui.selectable_label(selected, &pattern.name).clicked() {
            ui_state.selected_pattern = i;
            ui_state.current_tool = Tool::Place;
        }
    }

    ui.add_space(16.0);
    ui.separator();
    ui.heading("Import RLE");

    ui.add(
        egui::TextEdit::multiline(&mut ui_state.custom_pattern_rle)
            .hint_text("Paste RLE pattern here...")
            .desired_rows(4),
    );

    if ui.button("Load Pattern").clicked() {
        if let Some(_pattern) = Pattern::from_rle(&ui_state.custom_pattern_rle) {
            ui_state.current_tool = Tool::Place;
        }
    }
}

fn draw_neighborhood_panel(ui: &mut egui::Ui, ui_state: &mut UiState, sim: &mut Simulation) {
    ui.heading("Neighborhood");

    let neighborhoods = Neighborhood::all_standard();
    for (i, nbhd) in neighborhoods.iter().enumerate() {
        let selected = ui_state.selected_neighborhood == i;
        if ui
            .selectable_label(selected, &format!("{} ({})", nbhd.name, nbhd.size()))
            .clicked()
        {
            ui_state.selected_neighborhood = i;
            sim.set_neighborhood(nbhd.clone());
        }
    }

    ui.add_space(16.0);
    ui.separator();
    ui.heading("Custom Neighborhood");

    ui.label("Click cells to toggle:");

    let range = ui_state.neighborhood_editor_range;
    let _size = (range * 2 + 1) as usize;

    egui::Grid::new("neighborhood_editor")
        .spacing([2.0, 2.0])
        .show(ui, |ui| {
            for dy in -range..=range {
                for dx in -range..=range {
                    let is_center = dx == 0 && dy == 0;
                    let is_active = ui_state.custom_neighborhood_offsets.contains(&(dx, dy));

                    let color = if is_center {
                        egui::Color32::BLUE
                    } else if is_active {
                        egui::Color32::GREEN
                    } else {
                        egui::Color32::DARK_GRAY
                    };

                    let btn = egui::Button::new("")
                        .fill(color)
                        .min_size(egui::vec2(20.0, 20.0));

                    if ui.add(btn).clicked() && !is_center {
                        if is_active {
                            ui_state
                                .custom_neighborhood_offsets
                                .retain(|&o| o != (dx, dy));
                        } else {
                            ui_state.custom_neighborhood_offsets.push((dx, dy));
                        }
                    }
                }
                ui.end_row();
            }
        });

    ui.horizontal(|ui| {
        ui.label("Range:");
        if ui.button("-").clicked() && ui_state.neighborhood_editor_range > 1 {
            ui_state.neighborhood_editor_range -= 1;
        }
        ui.label(format!("{}", ui_state.neighborhood_editor_range));
        if ui.button("+").clicked() && ui_state.neighborhood_editor_range < 5 {
            ui_state.neighborhood_editor_range += 1;
        }
    });

    if ui.button("Apply Custom").clicked() && !ui_state.custom_neighborhood_offsets.is_empty() {
        let nbhd = Neighborhood::custom("Custom", ui_state.custom_neighborhood_offsets.clone());
        sim.set_neighborhood(nbhd);
    }
}

fn draw_statistics_panel(ui: &mut egui::Ui, _ui_state: &mut UiState, sim: &mut Simulation) {
    ui.heading("Statistics");

    let stats = sim.stats();

    ui.label(format!("Generation: {}", sim.generation()));
    ui.label(format!("Population: {}", stats.population));
    ui.label(format!("Density: {:.2}%", stats.density * 100.0));
    ui.label(format!("Entropy: {:.4} bits", stats.entropy));

    if let RuleSpec::LifeLike(ref rule) = sim.config.rule.spec {
        ui.label(format!("Lambda (λ): {:.4}", rule.lambda()));
    }

    ui.add_space(16.0);
    ui.separator();
    ui.heading("Population History");

    if !stats.population_history.is_empty() {
        let points: egui_plot::PlotPoints = stats
            .population_history
            .iter()
            .enumerate()
            .map(|(i, &p)| [i as f64, p as f64])
            .collect();

        let line = egui_plot::Line::new(points).name("Population");

        egui_plot::Plot::new("population_plot")
            .height(150.0)
            .show_axes(true)
            .show(ui, |plot_ui| {
                plot_ui.line(line);
            });
    }

    ui.add_space(8.0);
    ui.heading("Entropy History");

    if !stats.entropy_history.is_empty() {
        let points: egui_plot::PlotPoints = stats
            .entropy_history
            .iter()
            .enumerate()
            .map(|(i, &e)| [i as f64, e])
            .collect();

        let line = egui_plot::Line::new(points)
            .name("Entropy")
            .color(egui::Color32::GREEN);

        egui_plot::Plot::new("entropy_plot")
            .height(150.0)
            .show_axes(true)
            .show(ui, |plot_ui| {
                plot_ui.line(line);
            });
    }
}

fn draw_spacetime_panel(ui: &mut egui::Ui, ui_state: &mut UiState, sim: &mut Simulation) {
    ui.heading("Spacetime View");

    ui.horizontal(|ui| {
        ui.selectable_value(
            &mut ui_state.spacetime_view_mode,
            SpacetimeViewMode::Slice2D,
            "2D Slice",
        );
        ui.selectable_value(
            &mut ui_state.spacetime_view_mode,
            SpacetimeViewMode::Isometric3D,
            "3D View",
        );
    });

    ui.add_space(4.0);

    if ui.button("Clear History").clicked() {
        ui_state.spacetime_history.clear();
        ui_state.spacetime_3d_history.clear();
    }

    ui.add_space(4.0);
    ui.separator();

    match ui_state.spacetime_view_mode {
        SpacetimeViewMode::Slice2D => draw_spacetime_2d(ui, ui_state, sim),
        SpacetimeViewMode::Isometric3D => draw_spacetime_3d(ui, ui_state, sim),
    }
}

fn draw_spacetime_2d(ui: &mut egui::Ui, ui_state: &mut UiState, sim: &mut Simulation) {
    ui.label("Horizontal slice over time (Wolfram-style)");

    ui.horizontal(|ui| {
        ui.label("Slice Y:");
        ui.add(egui::Slider::new(
            &mut ui_state.spacetime_slice_y,
            0..=sim.height().saturating_sub(1),
        ));
    });

    let slice_y = ui_state
        .spacetime_slice_y
        .min(sim.height().saturating_sub(1));
    let width = sim.width() as usize;
    let mut row = Vec::with_capacity(width);
    for x in 0..width {
        row.push(sim.get_cell(x as u32, slice_y));
    }

    ui_state.spacetime_history.push(row);
    while ui_state.spacetime_history.len() > ui_state.spacetime_max_history {
        ui_state.spacetime_history.remove(0);
    }

    let available_width = ui.available_width();
    let available_height = 180.0f32;

    let history_len = ui_state.spacetime_history.len();
    if history_len == 0 || width == 0 {
        return;
    }

    let cell_width = (available_width / width as f32).max(1.0);
    let cell_height = (available_height / history_len as f32).max(1.0).min(4.0);

    let (response, painter) = ui.allocate_painter(
        egui::vec2(available_width, cell_height * history_len as f32),
        egui::Sense::hover(),
    );

    let rect = response.rect;
    painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(5, 5, 15));

    let alive_color = get_alive_color(ui_state.color_scheme);

    for (t, row) in ui_state.spacetime_history.iter().enumerate() {
        for (x, &cell) in row.iter().enumerate() {
            if cell > 0 {
                let px = rect.min.x + x as f32 * cell_width;
                let py = rect.min.y + t as f32 * cell_height;
                let cell_rect = egui::Rect::from_min_size(
                    egui::pos2(px, py),
                    egui::vec2(cell_width, cell_height),
                );
                painter.rect_filled(cell_rect, 0.0, alive_color);
            }
        }
    }

    ui.label(format!("{} generations | Time flows down", history_len));
}

fn draw_spacetime_3d(ui: &mut egui::Ui, ui_state: &mut UiState, sim: &mut Simulation) {
    ui.label("3D spacetime cube (X, Y, Time)");

    ui.horizontal(|ui| {
        ui.label("Rot X:");
        ui.add(
            egui::Slider::new(
                &mut ui_state.spacetime_rotation_x,
                0.0..=std::f32::consts::PI,
            )
            .show_value(false),
        );
        ui.label("Rot Y:");
        ui.add(
            egui::Slider::new(
                &mut ui_state.spacetime_rotation_y,
                0.0..=std::f32::consts::TAU,
            )
            .show_value(false),
        );
    });

    ui.horizontal(|ui| {
        ui.label("Scale:");
        ui.add(egui::Slider::new(&mut ui_state.spacetime_scale, 0.5..=3.0).show_value(false));
        ui.label("Depth:");
        ui.add(egui::DragValue::new(&mut ui_state.spacetime_3d_max_depth).range(10..=100));
    });

    let sample_rate = (sim.width() / 32).max(1) as usize;
    let sampled_width = (sim.width() as usize / sample_rate).max(1);
    let sampled_height = (sim.height() as usize / sample_rate).max(1);

    let mut frame = Vec::with_capacity(sampled_height);
    for sy in 0..sampled_height {
        let mut row = Vec::with_capacity(sampled_width);
        for sx in 0..sampled_width {
            let x = (sx * sample_rate) as u32;
            let y = (sy * sample_rate) as u32;
            row.push(sim.get_cell(x.min(sim.width() - 1), y.min(sim.height() - 1)));
        }
        frame.push(row);
    }

    ui_state.spacetime_3d_history.push(frame);
    while ui_state.spacetime_3d_history.len() > ui_state.spacetime_3d_max_depth {
        ui_state.spacetime_3d_history.remove(0);
    }

    let available_size = ui.available_width().min(250.0);

    let (response, painter) = ui.allocate_painter(
        egui::vec2(available_size, available_size),
        egui::Sense::drag(),
    );

    if response.dragged() {
        let delta = response.drag_delta();
        ui_state.spacetime_rotation_y += delta.x * 0.01;
        ui_state.spacetime_rotation_x =
            (ui_state.spacetime_rotation_x + delta.y * 0.01).clamp(0.1, std::f32::consts::PI - 0.1);
    }

    let rect = response.rect;
    let center = rect.center();
    painter.rect_filled(rect, 4.0, egui::Color32::from_rgb(8, 8, 20));

    let rot_x = ui_state.spacetime_rotation_x;
    let rot_y = ui_state.spacetime_rotation_y;
    let scale = ui_state.spacetime_scale * available_size * 0.3;

    let cos_x = rot_x.cos();
    let sin_x = rot_x.sin();
    let cos_y = rot_y.cos();
    let sin_y = rot_y.sin();

    let project = |x: f32, y: f32, z: f32| -> egui::Pos2 {
        let xr = x * cos_y - z * sin_y;
        let zr = x * sin_y + z * cos_y;
        let yr = y * cos_x - zr * sin_x;
        let zr2 = y * sin_x + zr * cos_x;

        let perspective = 1.0 / (1.0 + zr2 * 0.3);
        egui::pos2(
            center.x + xr * scale * perspective,
            center.y + yr * scale * perspective,
        )
    };

    let axis_color = egui::Color32::from_rgba_unmultiplied(100, 100, 100, 150);
    let origin = project(0.0, 0.0, 0.0);
    painter.line_segment(
        [origin, project(1.0, 0.0, 0.0)],
        egui::Stroke::new(1.0, egui::Color32::RED),
    );
    painter.line_segment(
        [origin, project(0.0, 1.0, 0.0)],
        egui::Stroke::new(1.0, egui::Color32::GREEN),
    );
    painter.line_segment(
        [origin, project(0.0, 0.0, 1.0)],
        egui::Stroke::new(1.0, egui::Color32::BLUE),
    );

    let cube_points = [
        (-1.0, -1.0, -1.0),
        (1.0, -1.0, -1.0),
        (1.0, 1.0, -1.0),
        (-1.0, 1.0, -1.0),
        (-1.0, -1.0, 1.0),
        (1.0, -1.0, 1.0),
        (1.0, 1.0, 1.0),
        (-1.0, 1.0, 1.0),
    ];
    let cube_edges = [
        (0, 1),
        (1, 2),
        (2, 3),
        (3, 0),
        (4, 5),
        (5, 6),
        (6, 7),
        (7, 4),
        (0, 4),
        (1, 5),
        (2, 6),
        (3, 7),
    ];
    for &(i, j) in &cube_edges {
        let p1 = cube_points[i];
        let p2 = cube_points[j];
        painter.line_segment(
            [project(p1.0, p1.1, p1.2), project(p2.0, p2.1, p2.2)],
            egui::Stroke::new(0.5, axis_color),
        );
    }

    let depth = ui_state.spacetime_3d_history.len();
    if depth == 0 || sampled_width == 0 || sampled_height == 0 {
        ui.label("Waiting for data...");
        return;
    }

    let alive_color = get_alive_color(ui_state.color_scheme);

    let mut points_to_draw: Vec<(f32, egui::Pos2, egui::Color32)> = Vec::new();

    for (t, frame) in ui_state.spacetime_3d_history.iter().enumerate() {
        let tz = -1.0 + 2.0 * (t as f32 / depth as f32);
        let time_alpha = 50 + (200.0 * (t as f32 / depth as f32)) as u8;

        for (y, row) in frame.iter().enumerate() {
            let ty = -1.0 + 2.0 * (y as f32 / sampled_height as f32);
            for (x, &cell) in row.iter().enumerate() {
                if cell > 0 {
                    let tx = -1.0 + 2.0 * (x as f32 / sampled_width as f32);
                    let pos = project(tx, ty, tz);

                    let depth_val = tx * sin_y + tz * cos_y;
                    let depth_val = ty * sin_x + depth_val * cos_x;

                    let color = egui::Color32::from_rgba_unmultiplied(
                        alive_color.r(),
                        alive_color.g(),
                        alive_color.b(),
                        time_alpha,
                    );

                    points_to_draw.push((depth_val, pos, color));
                }
            }
        }
    }

    points_to_draw.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

    let point_size = (scale * 0.05).max(1.5).min(4.0);
    for (_, pos, color) in points_to_draw {
        if rect.contains(pos) {
            painter.circle_filled(pos, point_size, color);
        }
    }

    ui.label(format!("{} frames | Drag to rotate", depth));
    ui.label("X=red Y=green Time=blue");
}

fn draw_settings_panel(ui: &mut egui::Ui, ui_state: &mut UiState, sim: &mut Simulation) {
    ui.heading("Settings");

    ui.checkbox(&mut ui_state.show_grid, "Show Grid");
    ui.checkbox(&mut ui_state.show_stats_overlay, "Stats Overlay");

    ui.add_space(8.0);
    ui.label("Color Scheme:");
    ui.horizontal(|ui| {
        ui.selectable_value(&mut ui_state.color_scheme, ColorScheme::Classic, "Classic");
        ui.selectable_value(&mut ui_state.color_scheme, ColorScheme::Heat, "Heat");
        ui.selectable_value(&mut ui_state.color_scheme, ColorScheme::Neon, "Neon");
    });
    ui.horizontal(|ui| {
        ui.selectable_value(&mut ui_state.color_scheme, ColorScheme::Ocean, "Ocean");
        ui.selectable_value(&mut ui_state.color_scheme, ColorScheme::Forest, "Forest");
        ui.selectable_value(&mut ui_state.color_scheme, ColorScheme::Grayscale, "Gray");
    });

    ui.add_space(16.0);
    ui.separator();
    ui.heading("Grid Settings");

    ui.horizontal(|ui| {
        ui.label("Width:");
        ui.add(egui::DragValue::new(&mut ui_state.grid_width_input).range(16..=1024));
    });

    ui.horizontal(|ui| {
        ui.label("Height:");
        ui.add(egui::DragValue::new(&mut ui_state.grid_height_input).range(16..=1024));
    });

    ui.horizontal(|ui| {
        ui.label("Seed:");
        ui.text_edit_singleline(&mut ui_state.seed_input);
    });

    ui.horizontal(|ui| {
        ui.label("Density:");
        ui.add(egui::Slider::new(&mut ui_state.density_input, 0.0..=1.0));
    });

    if ui.button("Apply & Reset").clicked() {
        let seed = ui_state.seed_input.parse().unwrap_or(0);
        let mut config = sim.config.clone();
        config.width = ui_state.grid_width_input;
        config.height = ui_state.grid_height_input;
        config.seed = seed;
        config.initial_density = ui_state.density_input;
        *sim = Simulation::new(config);
    }

    ui.add_space(16.0);
    ui.separator();
    ui.heading("Export / Import");

    if ui.button("Copy Config to Clipboard").clicked() {
        let config_str = sim.config.to_shareable_string();
        ui.ctx().copy_text(config_str);
    }

    if ui.button("Export Current State").clicked() {
        let state = sim.export_state();
        if let Ok(json) = serde_json::to_string_pretty(&state) {
            ui.ctx().copy_text(json);
        }
    }
}

fn draw_central_panel(ctx: &egui::Context, ui_state: &mut UiState, sim: &mut Simulation) {
    egui::CentralPanel::default().show(ctx, |ui| {
        let available_size = ui.available_size();
        let grid_width = sim.width() as f32;
        let grid_height = sim.height() as f32;

        let scale = (available_size.x / grid_width).min(available_size.y / grid_height);
        let cell_size = scale.max(1.0);

        let total_width = grid_width * cell_size;
        let total_height = grid_height * cell_size;

        let offset_x = (available_size.x - total_width) / 2.0;
        let offset_y = (available_size.y - total_height) / 2.0;

        let (response, painter) =
            ui.allocate_painter(available_size, egui::Sense::click_and_drag());
        let rect = response.rect;

        painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(5, 5, 15));

        let alive_color = get_alive_color(ui_state.color_scheme);
        let dead_color = get_dead_color(ui_state.color_scheme);

        for y in 0..sim.height() {
            for x in 0..sim.width() {
                let cell = sim.get_cell(x, y);
                if cell > 0 || ui_state.show_grid {
                    let px = rect.min.x + offset_x + x as f32 * cell_size;
                    let py = rect.min.y + offset_y + y as f32 * cell_size;

                    let cell_rect = egui::Rect::from_min_size(
                        egui::pos2(px, py),
                        egui::vec2(cell_size - 0.5, cell_size - 0.5),
                    );

                    let color = if cell > 0 { alive_color } else { dead_color };
                    painter.rect_filled(cell_rect, 0.0, color);
                }
            }
        }

        if ui_state.show_grid && cell_size > 3.0 {
            let grid_color = egui::Color32::from_rgba_unmultiplied(50, 50, 70, 100);
            for x in 0..=sim.width() {
                let px = rect.min.x + offset_x + x as f32 * cell_size;
                painter.line_segment(
                    [
                        egui::pos2(px, rect.min.y + offset_y),
                        egui::pos2(px, rect.min.y + offset_y + total_height),
                    ],
                    egui::Stroke::new(0.5, grid_color),
                );
            }
            for y in 0..=sim.height() {
                let py = rect.min.y + offset_y + y as f32 * cell_size;
                painter.line_segment(
                    [
                        egui::pos2(rect.min.x + offset_x, py),
                        egui::pos2(rect.min.x + offset_x + total_width, py),
                    ],
                    egui::Stroke::new(0.5, grid_color),
                );
            }
        }

        if response.clicked() || response.dragged() {
            if let Some(pos) = response.interact_pointer_pos() {
                let local_x = pos.x - rect.min.x - offset_x;
                let local_y = pos.y - rect.min.y - offset_y;

                if local_x >= 0.0 && local_y >= 0.0 {
                    let cell_x = (local_x / cell_size) as u32;
                    let cell_y = (local_y / cell_size) as u32;

                    if cell_x < sim.width() && cell_y < sim.height() {
                        match ui_state.current_tool {
                            Tool::Draw => sim.set_cell(cell_x, cell_y, 1),
                            Tool::Erase => sim.set_cell(cell_x, cell_y, 0),
                            Tool::Place => {
                                let all_patterns = patterns::all();
                                if ui_state.selected_pattern < all_patterns.len() {
                                    let pattern = &all_patterns[ui_state.selected_pattern];
                                    sim.place_pattern(pattern, cell_x, cell_y);
                                }
                            }
                            Tool::Pan => {}
                        }
                    }
                }
            }
        }
    });
}

fn get_alive_color(scheme: ColorScheme) -> egui::Color32 {
    match scheme {
        ColorScheme::Classic => egui::Color32::from_rgb(0, 255, 100),
        ColorScheme::Heat => egui::Color32::from_rgb(255, 100, 50),
        ColorScheme::Ocean => egui::Color32::from_rgb(50, 150, 255),
        ColorScheme::Forest => egui::Color32::from_rgb(50, 200, 50),
        ColorScheme::Neon => egui::Color32::from_rgb(255, 0, 255),
        ColorScheme::Grayscale => egui::Color32::from_rgb(220, 220, 220),
    }
}

fn get_dead_color(scheme: ColorScheme) -> egui::Color32 {
    match scheme {
        ColorScheme::Classic => egui::Color32::from_rgb(10, 20, 10),
        ColorScheme::Heat => egui::Color32::from_rgb(30, 10, 5),
        ColorScheme::Ocean => egui::Color32::from_rgb(5, 15, 30),
        ColorScheme::Forest => egui::Color32::from_rgb(10, 25, 10),
        ColorScheme::Neon => egui::Color32::from_rgb(20, 5, 20),
        ColorScheme::Grayscale => egui::Color32::from_rgb(20, 20, 20),
    }
}

fn draw_stats_overlay(ctx: &egui::Context, ui_state: &UiState, sim: &Simulation) {
    if !ui_state.show_stats_overlay {
        return;
    }

    egui::Area::new(egui::Id::new("stats_overlay"))
        .anchor(egui::Align2::RIGHT_TOP, [-10.0, 40.0])
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style())
                .fill(egui::Color32::from_rgba_unmultiplied(0, 0, 0, 200))
                .show(ui, |ui| {
                    let stats = sim.stats();
                    ui.label(format!("Gen: {}", sim.generation()));
                    ui.label(format!(
                        "Pop: {} ({:.1}%)",
                        stats.population,
                        stats.density * 100.0
                    ));
                    ui.label(format!("H: {:.3}", stats.entropy));
                });
        });
}
