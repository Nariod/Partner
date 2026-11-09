use crate::i18n::strings;
use eframe::egui::{self, RichText};
use partner_core::model::NodeId;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Command {
    NewNote,
    NewSubNote,
    Save,
    SaveAs,
    Lock,
    TogglePreview,
    ToggleSidebar,
    PasteImage,
    Settings,
    About,
    CheckUpdates,
    GenerateKeypair,
}

pub fn command_list() -> Vec<(&'static str, Command)> {
    vec![
        ("New note", Command::NewNote),
        ("New sub-note", Command::NewSubNote),
        ("Save", Command::Save),
        ("Save as…", Command::SaveAs),
        ("Lock now", Command::Lock),
        ("Toggle preview", Command::TogglePreview),
        ("Toggle sidebar", Command::ToggleSidebar),
        ("Paste image from clipboard", Command::PasteImage),
        ("Settings", Command::Settings),
        ("Check for updates", Command::CheckUpdates),
        ("Generate age keypair", Command::GenerateKeypair),
        ("About", Command::About),
    ]
}

pub struct Switcher {
    pub filter: String,
    pub selected: usize,
    pub focus: bool,
}

pub struct Palette {
    pub filter: String,
    pub selected: usize,
    pub focus: bool,
}

pub struct NewDocForm {
    pub title: String,
    pub path: Option<PathBuf>,
    pub pass1: String,
    pub pass2: String,
    pub armor: bool,
    pub error: Option<String>,
}

impl Default for NewDocForm {
    fn default() -> Self {
        Self {
            title: "Mission".to_string(),
            path: None,
            pass1: String::new(),
            pass2: String::new(),
            armor: false,
            error: None,
        }
    }
}

pub enum Dialog {
    New(NewDocForm),
    Settings,
    About,
    Keypair { secret: String, public: String },
}

pub enum OverlayAction {
    SelectNode(NodeId),
    Run(Command),
    CloseOverlay,
    Dialog(DialogAction),
}

pub enum DialogAction {
    Create,
    SaveSettings,
    BrowseLocation,
    GenerateKeypair,
    Close,
}

pub fn switcher_window(
    ctx: &egui::Context,
    sw: &mut Switcher,
    nodes: &[(NodeId, String)],
    out: &mut Vec<OverlayAction>,
) {
    let mut open = true;
    egui::Window::new(strings().quick_switcher)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, -120.0))
        .default_width(480.0)
        .open(&mut open)
        .show(ctx, |ui| {
            let id = ui.scope_id().with("sw_filter");
            egui::TextEdit::singleline(&mut sw.filter)
                .hint_text(strings().type_to_filter)
                .desired_width(f32::INFINITY)
                .show(ui);
            if sw.focus {
                ui.memory_mut(|m| m.request_focus(id));
                sw.focus = false;
            }
            let filter = sw.filter.to_lowercase();
            let matches: Vec<(NodeId, String)> = nodes
                .iter()
                .filter(|(_, t)| filter.is_empty() || t.to_lowercase().contains(&filter))
                .cloned()
                .collect();
            if sw.selected >= matches.len() {
                sw.selected = matches.len().saturating_sub(1);
            }
            egui::ScrollArea::vertical()
                .max_height(320.0)
                .show(ui, |ui| {
                    for (i, (id, title)) in matches.iter().enumerate() {
                        if ui.selectable_label(i == sw.selected, title).clicked() {
                            out.push(OverlayAction::SelectNode(*id));
                            out.push(OverlayAction::CloseOverlay);
                        }
                    }
                });
            ui.input(|i| {
                if i.key_pressed(egui::Key::ArrowDown) {
                    sw.selected = (sw.selected + 1).min(matches.len().saturating_sub(1));
                }
                if i.key_pressed(egui::Key::ArrowUp) && sw.selected > 0 {
                    sw.selected -= 1;
                }
                if i.key_pressed(egui::Key::Enter) {
                    if let Some((id, _)) = matches.get(sw.selected) {
                        out.push(OverlayAction::SelectNode(*id));
                    }
                    out.push(OverlayAction::CloseOverlay);
                }
                if i.key_pressed(egui::Key::Escape) {
                    out.push(OverlayAction::CloseOverlay);
                }
            });
        });
    if !open {
        out.push(OverlayAction::CloseOverlay);
    }
}

pub fn palette_window(ctx: &egui::Context, pal: &mut Palette, out: &mut Vec<OverlayAction>) {
    let mut open = true;
    let cmds = command_list();
    egui::Window::new(strings().command_palette)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, -140.0))
        .default_width(480.0)
        .open(&mut open)
        .show(ctx, |ui| {
            let id = ui.scope_id().with("pal_filter");
            egui::TextEdit::singleline(&mut pal.filter)
                .hint_text(strings().type_to_filter)
                .desired_width(f32::INFINITY)
                .show(ui);
            if pal.focus {
                ui.memory_mut(|m| m.request_focus(id));
                pal.focus = false;
            }
            let filter = pal.filter.to_lowercase();
            let matches: Vec<(&'static str, Command)> = cmds
                .into_iter()
                .filter(|(label, _)| filter.is_empty() || label.to_lowercase().contains(&filter))
                .collect();
            if pal.selected >= matches.len() {
                pal.selected = matches.len().saturating_sub(1);
            }
            egui::ScrollArea::vertical()
                .max_height(320.0)
                .show(ui, |ui| {
                    for (i, (label, cmd)) in matches.iter().enumerate() {
                        if ui.selectable_label(i == pal.selected, *label).clicked() {
                            out.push(OverlayAction::Run(*cmd));
                            out.push(OverlayAction::CloseOverlay);
                        }
                    }
                });
            ui.input(|i| {
                if i.key_pressed(egui::Key::ArrowDown) {
                    pal.selected = (pal.selected + 1).min(matches.len().saturating_sub(1));
                }
                if i.key_pressed(egui::Key::ArrowUp) && pal.selected > 0 {
                    pal.selected -= 1;
                }
                if i.key_pressed(egui::Key::Enter) {
                    if let Some((_, cmd)) = matches.get(pal.selected) {
                        out.push(OverlayAction::Run(*cmd));
                    }
                    out.push(OverlayAction::CloseOverlay);
                }
                if i.key_pressed(egui::Key::Escape) {
                    out.push(OverlayAction::CloseOverlay);
                }
            });
        });
    if !open {
        out.push(OverlayAction::CloseOverlay);
    }
}

pub fn dialog_window(
    ctx: &egui::Context,
    dialog: &mut Dialog,
    settings: &mut crate::config::Settings,
    out: &mut Vec<OverlayAction>,
) {
    match dialog {
        Dialog::New(form) => {
            let mut open = true;
            egui::Window::new(strings().new_document)
                .collapsible(false)
                .resizable(false)
                .default_width(460.0)
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(strings().title);
                        ui.add(
                            egui::TextEdit::singleline(&mut form.title)
                                .desired_width(f32::INFINITY),
                        );
                    });
                    ui.horizontal(|ui| {
                        ui.label(strings().location);
                        let label = form
                            .path
                            .as_ref()
                            .map(|p| p.display().to_string())
                            .unwrap_or_else(|| "…".to_string());
                        ui.label(RichText::new(label).weak());
                        if ui.button(strings().browse).clicked() {
                            out.push(OverlayAction::Dialog(DialogAction::BrowseLocation));
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label(strings().passphrase);
                        ui.add(
                            egui::TextEdit::singleline(&mut form.pass1)
                                .password(true)
                                .desired_width(f32::INFINITY),
                        );
                    });
                    ui.horizontal(|ui| {
                        ui.label(strings().confirm_passphrase);
                        ui.add(
                            egui::TextEdit::singleline(&mut form.pass2)
                                .password(true)
                                .desired_width(f32::INFINITY),
                        );
                    });
                    ui.checkbox(&mut form.armor, strings().ascii_armor);
                    if let Some(e) = &form.error {
                        ui.label(RichText::new(e).color(egui::Color32::RED));
                    }
                    ui.horizontal(|ui| {
                        if ui.button(strings().create).clicked() {
                            out.push(OverlayAction::Dialog(DialogAction::Create));
                        }
                        if ui.button(strings().cancel).clicked() {
                            out.push(OverlayAction::Dialog(DialogAction::Close));
                        }
                    });
                });
            if !open {
                out.push(OverlayAction::Dialog(DialogAction::Close));
            }
        }
        Dialog::Settings => {
            let mut open = true;
            egui::Window::new(strings().settings)
                .collapsible(false)
                .resizable(false)
                .default_width(440.0)
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(strings().autolock_minutes);
                        ui.add(egui::DragValue::new(&mut settings.autolock_minutes).range(0..=240));
                    });
                    ui.horizontal(|ui| {
                        ui.label(strings().autosave_seconds);
                        ui.add(egui::DragValue::new(&mut settings.autosave_seconds).range(0..=600));
                    });
                    ui.horizontal(|ui| {
                        ui.label(strings().backups_count);
                        ui.add(egui::DragValue::new(&mut settings.backups).range(0..=20));
                    });
                    ui.checkbox(&mut settings.armor, strings().ascii_armor);
                    ui.checkbox(&mut settings.check_updates, strings().enable_update_check);
                    ui.horizontal(|ui| {
                        if ui.button(strings().save).clicked() {
                            out.push(OverlayAction::Dialog(DialogAction::SaveSettings));
                            out.push(OverlayAction::Dialog(DialogAction::Close));
                        }
                        if ui.button(strings().close).clicked() {
                            out.push(OverlayAction::Dialog(DialogAction::Close));
                        }
                    });
                });
            if !open {
                out.push(OverlayAction::Dialog(DialogAction::Close));
            }
        }
        Dialog::About => {
            let mut open = true;
            egui::Window::new(strings().about)
                .collapsible(false)
                .resizable(false)
                .default_width(440.0)
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.heading(format!("{} v{}", strings().app_name, env!("CARGO_PKG_VERSION")));
                    ui.label(strings().tagline);
                    ui.add_space(6.0);
                    ui.label("Format: one .partner file per mission, encrypted with age-encryption.org/v1.");
                    ui.label("License: AGPL-3.0 — Source: github.com/Nariod/Partner");
                    ui.horizontal(|ui| {
                        if ui.button(strings().generate_keypair).clicked() {
                            out.push(OverlayAction::Dialog(DialogAction::GenerateKeypair));
                        }
                        if ui.button(strings().close).clicked() {
                            out.push(OverlayAction::Dialog(DialogAction::Close));
                        }
                    });
                });
            if !open {
                out.push(OverlayAction::Dialog(DialogAction::Close));
            }
        }
        Dialog::Keypair { secret, public } => {
            let mut open = true;
            egui::Window::new(strings().generate_keypair)
                .collapsible(false)
                .resizable(false)
                .default_width(580.0)
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.label(RichText::new(strings().secret_key).strong());
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(secret.as_str()).family(egui::FontFamily::Monospace),
                        );
                        if ui.button(strings().copy).clicked() {
                            ui.ctx().copy_text(secret.clone());
                        }
                    });
                    ui.add_space(6.0);
                    ui.label(RichText::new(strings().public_key).strong());
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(public.as_str()).family(egui::FontFamily::Monospace),
                        );
                        if ui.button(strings().copy).clicked() {
                            ui.ctx().copy_text(public.clone());
                        }
                    });
                    ui.horizontal(|ui| {
                        if ui.button(strings().close).clicked() {
                            out.push(OverlayAction::Dialog(DialogAction::Close));
                        }
                    });
                });
            if !open {
                out.push(OverlayAction::Dialog(DialogAction::Close));
            }
        }
    }
}
