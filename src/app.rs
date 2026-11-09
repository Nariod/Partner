use crate::{
    config::Config,
    editor,
    i18n::strings,
    overlays::{self, Command, Dialog, DialogAction, OverlayAction, Palette, Switcher},
    session::{OpenDoc, Screen},
    tree::{self, TreeAction},
    updater::{self, UpdateMsg, UpdateState},
};
use eframe::egui::{self, RichText};
use partner_core::{
    crypto::{
        decrypt_document, encrypt_document, generate_keypair, identity_secret_string, Credential,
    },
    model::{Document, NodeId},
    storage,
};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::mpsc::{channel, Receiver, Sender},
    thread,
    time::{Duration, Instant},
};

pub const APP_ID: &str = "io.github.Nariod.Partner";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub enum Action {
    NewDocument,
    OpenDocument,
    OpenRecent(PathBuf),
    Save,
    SaveAs,
    Lock,
    Quit,
    Unlock,
    CancelUnlock,
    NewNote,
    NewSubNote,
    DeleteSelected,
    TogglePreview,
    ToggleSidebar,
    OpenSwitcher,
    OpenPalette,
    OpenSettings,
    OpenAbout,
    PasteImage,
    CheckUpdates,
    UpdateDownload,
    UpdateInstall,
    UpdateDismiss,
}

pub struct PartnerApp {
    screen: Screen,
    open: Option<OpenDoc>,
    config: Config,
    search: String,
    show_sidebar: bool,
    show_preview: bool,
    renaming: Option<NodeId>,
    last_cursor: Option<egui::text::CCursorRange>,
    actions: Vec<Action>,
    tree_actions: Vec<TreeAction>,
    switcher: Option<Switcher>,
    palette: Option<Palette>,
    dialog: Option<Dialog>,
    status: String,
    last_activity: Instant,
    last_autosave: Instant,
    textures: HashMap<String, egui::TextureHandle>,
    update_tx: Sender<UpdateMsg>,
    update_rx: Receiver<UpdateMsg>,
    update_state: UpdateState,
    update_running: bool,
    last_update_check: Option<Instant>,
}

impl PartnerApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx.set_visuals(egui::Visuals::dark());
        let (tx, rx) = channel();
        Self {
            screen: Screen::Welcome,
            open: None,
            config: Config::load(),
            search: String::new(),
            show_sidebar: true,
            show_preview: true,
            renaming: None,
            last_cursor: None,
            actions: Vec::new(),
            tree_actions: Vec::new(),
            switcher: None,
            palette: None,
            dialog: None,
            status: String::new(),
            last_activity: Instant::now(),
            last_autosave: Instant::now(),
            textures: HashMap::new(),
            update_tx: tx,
            update_rx: rx,
            update_state: UpdateState::Idle,
            update_running: false,
            last_update_check: None,
        }
    }

    fn poll_activity(&mut self, ui: &egui::Ui) {
        if ui.input(|i| !i.events.is_empty()) {
            self.last_activity = Instant::now();
        }
    }

    fn handle_shortcuts(&mut self, ui: &egui::Ui) {
        ui.input(|i| {
            let cmd = i.modifiers.command;
            if cmd && i.modifiers.shift && i.key_pressed(egui::Key::P) {
                self.actions.push(Action::OpenPalette);
            } else if cmd && i.key_pressed(egui::Key::P) {
                self.actions.push(Action::OpenSwitcher);
            } else if cmd && i.key_pressed(egui::Key::S) {
                self.actions.push(Action::Save);
            } else if cmd && i.key_pressed(egui::Key::N) {
                self.actions.push(Action::NewNote);
            } else if cmd && i.key_pressed(egui::Key::L) {
                self.actions.push(Action::Lock);
            } else if cmd && i.key_pressed(egui::Key::E) {
                self.actions.push(Action::TogglePreview);
            } else if cmd && i.key_pressed(egui::Key::O) {
                self.actions.push(Action::OpenDocument);
            }
        });
    }

    fn drain_update_msgs(&mut self) {
        while let Ok(msg) = self.update_rx.try_recv() {
            self.update_running = false;
            match msg {
                UpdateMsg::NoUpdate => self.update_state = UpdateState::NoUpdate,
                UpdateMsg::Available(rel) => self.update_state = UpdateState::Available(rel),
                UpdateMsg::Downloaded { version, path } => {
                    self.update_state = UpdateState::Downloaded { version, path };
                }
                UpdateMsg::Error(e) => self.update_state = UpdateState::Error(e),
            }
        }
    }

    fn run_timers(&mut self) {
        let now = Instant::now();
        if matches!(self.screen, Screen::Editor) {
            let (autosave, autolock) = (
                self.config.settings.autosave_seconds,
                self.config.settings.autolock_minutes,
            );
            if autosave > 0
                && self.open.as_ref().map(|o| o.dirty).unwrap_or(false)
                && now.duration_since(self.last_autosave) > Duration::from_secs(autosave)
            {
                self.last_autosave = now;
                self.do_save();
            }
            if autolock > 0
                && now.duration_since(self.last_activity) > Duration::from_secs(autolock * 60)
            {
                self.do_lock();
            }
        }
        if self.config.settings.check_updates && !updater::is_flatpak() && !self.update_running {
            let due = self
                .last_update_check
                .map(|t| now.duration_since(t) > Duration::from_secs(24 * 3600))
                .unwrap_or(true);
            if due {
                self.last_update_check = Some(now);
                self.update_running = true;
                let tx = self.update_tx.clone();
                let v = VERSION.to_string();
                thread::spawn(move || updater::check_for_update(&v, tx));
            }
        }
    }

    fn ui_welcome(&mut self, ui: &mut egui::Ui) {
        let s = strings();
        egui::Panel::top("mb").show(ui, |ui| self.menu_bar(ui, false));
        egui::CentralPanel::default().show(ui, |ui| {
            ui.add_space(70.0);
            ui.vertical_centered(|ui| {
                ui.heading(RichText::new(s.welcome_title).size(52.0));
                ui.label(RichText::new(s.tagline).weak());
                ui.label(RichText::new(s.welcome_hint).weak());
                ui.add_space(24.0);
                if ui
                    .add_sized([220.0, 36.0], egui::Button::new(s.new_file))
                    .clicked()
                {
                    self.actions.push(Action::NewDocument);
                }
                if ui
                    .add_sized([220.0, 36.0], egui::Button::new(s.open_file))
                    .clicked()
                {
                    self.actions.push(Action::OpenDocument);
                }
                ui.add_space(30.0);
                ui.label(RichText::new(s.recent_files).strong());
                let recents = self.config.recent.clone();
                if recents.is_empty() {
                    ui.label(RichText::new(s.no_recent).weak());
                }
                for p in recents {
                    if ui.link(p.display().to_string()).clicked() {
                        self.actions.push(Action::OpenRecent(p));
                    }
                }
            });
        });
    }

    fn ui_unlock(&mut self, ui: &mut egui::Ui) {
        let s = strings();
        egui::Panel::top("mb").show(ui, |ui| self.menu_bar(ui, false));
        let mut acts: Vec<Action> = Vec::new();
        if let Screen::Unlock {
            path,
            pass,
            key,
            use_key,
            error,
            ..
        } = &mut self.screen
        {
            egui::CentralPanel::default().show(ui, |ui| {
                ui.add_space(90.0);
                ui.vertical_centered(|ui| {
                    ui.heading(format!("🔒 {}", s.unlock_title));
                    ui.label(RichText::new(path.display().to_string()).weak());
                    ui.add_space(8.0);
                    ui.label(RichText::new(s.unlock_hint).weak());
                    ui.add_space(12.0);
                    ui.checkbox(use_key, s.use_key_instead);
                    if *use_key {
                        ui.add(
                            egui::TextEdit::singleline(key)
                                .password(true)
                                .desired_width(380.0)
                                .hint_text(s.age_secret_key),
                        );
                    } else {
                        ui.add(
                            egui::TextEdit::singleline(pass)
                                .password(true)
                                .desired_width(380.0)
                                .hint_text(s.passphrase),
                        );
                    }
                    if let Some(e) = error {
                        ui.label(RichText::new(e.as_str()).color(egui::Color32::RED));
                    }
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        if ui.button(s.unlock).clicked() {
                            acts.push(Action::Unlock);
                        }
                        if ui.button(s.back).clicked() {
                            acts.push(Action::CancelUnlock);
                        }
                    });
                });
            });
        }
        self.actions.extend(acts);
    }

    fn ui_editor(&mut self, ui: &mut egui::Ui) {
        egui::Panel::top("menubar").show(ui, |ui| self.menu_bar(ui, true));
        if self.show_sidebar {
            egui::Panel::left("sidebar")
                .resizable(true)
                .default_size(260.0)
                .min_size(160.0)
                .show(ui, |ui| self.sidebar(ui));
        }
        egui::Panel::bottom("statusbar").show(ui, |ui| self.status_bar(ui));
        egui::CentralPanel::default().show(ui, |ui| self.central(ui));
    }

    fn menu_bar(&mut self, ui: &mut egui::Ui, in_editor: bool) {
        let s = strings();
        ui.horizontal(|ui| {
            ui.menu_button(s.file, |ui| {
                if ui.button(s.new_file).clicked() {
                    self.actions.push(Action::NewDocument);
                    ui.close();
                }
                if ui.button(s.open_file).clicked() {
                    self.actions.push(Action::OpenDocument);
                    ui.close();
                }
                if in_editor {
                    ui.separator();
                    if ui.button(s.save).clicked() {
                        self.actions.push(Action::Save);
                        ui.close();
                    }
                    if ui.button(s.save_as).clicked() {
                        self.actions.push(Action::SaveAs);
                        ui.close();
                    }
                    ui.separator();
                    if ui.button(s.lock_now).clicked() {
                        self.actions.push(Action::Lock);
                        ui.close();
                    }
                }
                ui.separator();
                if ui.button(s.quit).clicked() {
                    self.actions.push(Action::Quit);
                    ui.close();
                }
            });
            if in_editor {
                ui.menu_button(s.edit, |ui| {
                    if ui.button(s.new_note).clicked() {
                        self.actions.push(Action::NewNote);
                        ui.close();
                    }
                    if ui.button(s.new_subnote).clicked() {
                        self.actions.push(Action::NewSubNote);
                        ui.close();
                    }
                    if ui.button(s.delete_selected).clicked() {
                        self.actions.push(Action::DeleteSelected);
                        ui.close();
                    }
                    if ui.button(s.paste_image).clicked() {
                        self.actions.push(Action::PasteImage);
                        ui.close();
                    }
                });
                ui.menu_button(s.view, |ui| {
                    if ui.button(s.toggle_preview).clicked() {
                        self.actions.push(Action::TogglePreview);
                        ui.close();
                    }
                    if ui.button(s.toggle_sidebar).clicked() {
                        self.actions.push(Action::ToggleSidebar);
                        ui.close();
                    }
                });
            }
            ui.menu_button(s.help, |ui| {
                if ui.button(s.settings).clicked() {
                    self.actions.push(Action::OpenSettings);
                    ui.close();
                }
                if ui.button(s.check_updates).clicked() {
                    self.actions.push(Action::CheckUpdates);
                    ui.close();
                }
                if ui.button(s.about).clicked() {
                    self.actions.push(Action::OpenAbout);
                    ui.close();
                }
            });
        });
    }

    fn sidebar(&mut self, ui: &mut egui::Ui) {
        let s = strings();
        ui.horizontal(|ui| {
            if ui.button(s.new_note).clicked() {
                self.actions.push(Action::NewNote);
            }
        });
        ui.add(
            egui::TextEdit::singleline(&mut self.search)
                .hint_text(s.search_placeholder)
                .desired_width(f32::INFINITY),
        );
        ui.separator();
        egui::ScrollArea::vertical().show(ui, |ui| {
            if let Some(open) = &mut self.open {
                let mut ctx = tree::TreeCtx {
                    doc: &mut open.doc,
                    selected: &mut open.selected,
                    renaming: &mut self.renaming,
                    search: &self.search,
                    dirty: &mut open.dirty,
                    actions: &mut self.tree_actions,
                };
                tree::render_tree(ui, &mut ctx);
            }
        });
    }

    fn status_bar(&mut self, ui: &mut egui::Ui) {
        let s = strings();
        ui.horizontal(|ui| {
            if ui.small_button("🔒").on_hover_text(s.lock_now).clicked() {
                self.actions.push(Action::Lock);
            }
            if let Some(open) = &self.open {
                ui.label(RichText::new(open.path.display().to_string()).small());
                if open.dirty {
                    ui.label(
                        RichText::new(format!("● {}", s.unsaved_changes))
                            .color(egui::Color32::from_rgb(255, 170, 0))
                            .small(),
                    );
                }
                ui.separator();
                ui.label(
                    RichText::new(format!(
                        "{} {}",
                        open.doc.word_count(open.selected),
                        s.words
                    ))
                    .small(),
                );
            } else {
                ui.label(RichText::new(s.no_file_open).small());
            }
            ui.separator();
            ui.label(RichText::new(self.status.as_str()).small().weak());
        });
    }

    fn central(&mut self, ui: &mut egui::Ui) {
        let s = strings();
        match &self.update_state {
            UpdateState::Available(rel) => {
                egui::Frame::window(ui.style())
                    .fill(egui::Color32::from_rgb(90, 70, 10))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(format!("{} {}", s.update_available, rel.tag_name));
                            if ui.button(s.update_download).clicked() {
                                self.actions.push(Action::UpdateDownload);
                            }
                            if ui.button(s.update_dismiss).clicked() {
                                self.actions.push(Action::UpdateDismiss);
                            }
                        });
                    });
            }
            UpdateState::Downloaded { version, path } => {
                egui::Frame::window(ui.style())
                    .fill(egui::Color32::from_rgb(10, 70, 30))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(format!(
                                "{} {} — {}",
                                s.update_ready,
                                version,
                                path.display()
                            ));
                            if cfg!(windows) && ui.button(s.update_install).clicked() {
                                self.actions.push(Action::UpdateInstall);
                            }
                        });
                    });
            }
            UpdateState::Error(e) => {
                ui.label(
                    RichText::new(format!("{}: {}", s.update_failed, e))
                        .small()
                        .color(egui::Color32::from_rgb(200, 80, 80)),
                );
            }
            _ => {}
        }
        let Self {
            open,
            show_preview,
            textures,
            last_cursor,
            ..
        } = self;
        let Some(open) = open else {
            ui.centered_and_justified(|ui| {
                ui.label(s.no_file_open);
            });
            return;
        };
        if !open.doc.nodes.contains_key(&open.selected) {
            open.selected = open.doc.meta.root;
        }
        let id = open.selected;
        if id == open.doc.meta.root {
            ui.centered_and_justified(|ui| {
                ui.label("Select or create a note in the sidebar");
            });
            return;
        }
        if *show_preview {
            let body = open.doc.nodes[&id].body.clone();
            egui::Panel::right("preview")
                .resizable(true)
                .default_size(380.0)
                .min_size(180.0)
                .show(ui, |ui| {
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            editor::render_markdown(ui, &body, &open.doc, textures);
                        });
                });
        }
        editor::render_editor(ui, &mut open.doc, id, &mut open.dirty, last_cursor);
    }

    fn render_overlays(&mut self, ctx: &egui::Context) {
        let mut oa: Vec<OverlayAction> = Vec::new();
        if let Some(sw) = &mut self.switcher {
            let nodes: Vec<(NodeId, String)> = match &self.open {
                Some(o) => o
                    .doc
                    .all_node_ids()
                    .into_iter()
                    .map(|id| (id, o.doc.nodes[&id].title.clone()))
                    .collect(),
                None => Vec::new(),
            };
            overlays::switcher_window(ctx, sw, &nodes, &mut oa);
        }
        if let Some(pal) = &mut self.palette {
            overlays::palette_window(ctx, pal, &mut oa);
        }
        if let Some(d) = &mut self.dialog {
            overlays::dialog_window(ctx, d, &mut self.config.settings, &mut oa);
        }
        self.execute_overlay_actions(oa);
    }

    fn execute_overlay_actions(&mut self, oa: Vec<OverlayAction>) {
        for a in oa {
            match a {
                OverlayAction::SelectNode(id) => {
                    if let Some(open) = &mut self.open {
                        open.selected = id;
                    }
                }
                OverlayAction::Run(cmd) => self.actions.push(match cmd {
                    Command::NewNote => Action::NewNote,
                    Command::NewSubNote => Action::NewSubNote,
                    Command::Save => Action::Save,
                    Command::SaveAs => Action::SaveAs,
                    Command::Lock => Action::Lock,
                    Command::TogglePreview => Action::TogglePreview,
                    Command::ToggleSidebar => Action::ToggleSidebar,
                    Command::PasteImage => Action::PasteImage,
                    Command::Settings => Action::OpenSettings,
                    Command::About => Action::OpenAbout,
                    Command::CheckUpdates => Action::CheckUpdates,
                    Command::GenerateKeypair => {
                        let (id, rec) = generate_keypair();
                        self.dialog = Some(Dialog::Keypair {
                            secret: identity_secret_string(&id),
                            public: rec.to_string(),
                        });
                        continue;
                    }
                }),
                OverlayAction::CloseOverlay => {
                    self.switcher = None;
                    self.palette = None;
                }
                OverlayAction::Dialog(da) => match da {
                    DialogAction::Create => self.do_create_document(),
                    DialogAction::SaveSettings => self.config.save(),
                    DialogAction::BrowseLocation => {
                        let p = rfd::FileDialog::new()
                            .add_filter("Partner", &["partner"])
                            .set_file_name("mission.partner")
                            .save_file();
                        if let (Some(Dialog::New(form)), Some(p)) = (&mut self.dialog, p) {
                            form.path = Some(p);
                        }
                    }
                    DialogAction::GenerateKeypair => {
                        let (id, rec) = generate_keypair();
                        self.dialog = Some(Dialog::Keypair {
                            secret: identity_secret_string(&id),
                            public: rec.to_string(),
                        });
                    }
                    DialogAction::Close => self.dialog = None,
                },
            }
        }
    }

    fn do_create_document(&mut self) {
        let Some(Dialog::New(form)) = &mut self.dialog else {
            return;
        };
        if form.title.trim().is_empty() || form.path.is_none() || form.pass1.is_empty() {
            form.error = Some(strings().missing_fields.to_string());
            return;
        }
        if form.pass1 != form.pass2 {
            form.error = Some(strings().passphrases_mismatch.to_string());
            return;
        }
        let title = form.title.clone();
        let path = form.path.clone().unwrap();
        let pass = form.pass1.clone();
        let armor = form.armor;
        let backups = self.config.settings.backups;
        let doc = Document::new(title);
        let creds = vec![Credential::passphrase(pass)];
        let result = encrypt_document(&doc, &creds, armor)
            .and_then(|data| storage::save_atomic(&path, &data, backups))
            .and_then(|_| storage::acquire_lock(&path));
        match result {
            Ok(lock) => {
                let selected = doc
                    .all_node_ids()
                    .into_iter()
                    .next()
                    .unwrap_or(doc.meta.root);
                self.config.add_recent(path.clone());
                self.open = Some(OpenDoc {
                    path,
                    doc,
                    creds,
                    dirty: false,
                    selected,
                    lock,
                });
                self.screen = Screen::Editor;
                self.dialog = None;
                self.status.clear();
            }
            Err(e) => {
                if let Some(Dialog::New(form)) = &mut self.dialog {
                    form.error = Some(e.to_string());
                }
            }
        }
    }

    fn do_open_path(&mut self, path: PathBuf) {
        if !path.exists() {
            self.status = format!("not found: {}", path.display());
            return;
        }
        match storage::acquire_lock(&path) {
            Ok(lock) => {
                self.config.add_recent(path.clone());
                self.screen = Screen::Unlock {
                    path,
                    lock,
                    pass: String::new(),
                    key: String::new(),
                    use_key: false,
                    error: None,
                };
            }
            Err(e) => self.status = e.to_string(),
        }
    }

    fn do_unlock(&mut self) {
        let Screen::Unlock {
            path,
            lock,
            pass,
            key,
            use_key,
            ..
        } = std::mem::replace(&mut self.screen, Screen::Welcome)
        else {
            return;
        };
        let creds = if use_key {
            vec![Credential::key(key.trim().to_string())]
        } else {
            vec![Credential::passphrase(pass)]
        };
        let result = storage::load(&path).and_then(|data| decrypt_document(&data, &creds));
        match result {
            Ok(doc) => {
                let selected = doc
                    .all_node_ids()
                    .into_iter()
                    .next()
                    .unwrap_or(doc.meta.root);
                self.config.add_recent(path.clone());
                self.status = format!("Unlocked {}", path.display());
                self.open = Some(OpenDoc {
                    path,
                    doc,
                    creds,
                    dirty: false,
                    selected,
                    lock,
                });
                self.screen = Screen::Editor;
            }
            Err(e) => {
                let msg = match &e {
                    partner_core::Error::BadCredentials => strings().wrong_credentials.to_string(),
                    other => other.to_string(),
                };
                self.screen = Screen::Unlock {
                    path,
                    lock,
                    pass: String::new(),
                    key: String::new(),
                    use_key,
                    error: Some(msg),
                };
            }
        }
    }

    fn do_lock(&mut self) {
        if let Some(open) = self.open.take() {
            let path = open.path.clone();
            self.screen = Screen::Unlock {
                path,
                lock: open.lock,
                pass: String::new(),
                key: String::new(),
                use_key: false,
                error: None,
            };
            self.renaming = None;
            self.switcher = None;
            self.palette = None;
            self.status = strings().locked_msg.to_string();
        }
    }

    fn do_save(&mut self) {
        let Some(open) = &self.open else {
            return;
        };
        let path = open.path.clone();
        let armor = self.config.settings.armor;
        let backups = self.config.settings.backups;
        let result = encrypt_document(&open.doc, &open.creds, armor)
            .and_then(|data| storage::save_atomic(&path, &data, backups));
        match result {
            Ok(()) => {
                if let Some(open) = &mut self.open {
                    open.dirty = false;
                }
                self.status = format!("{} {}", strings().saved, path.display());
            }
            Err(e) => self.status = format!("Save error: {e}"),
        }
    }

    fn do_save_as(&mut self) {
        let Some(open) = &self.open else {
            return;
        };
        let Some(path) = rfd::FileDialog::new()
            .add_filter("Partner", &["partner"])
            .set_file_name("mission.partner")
            .save_file()
        else {
            return;
        };
        let armor = self.config.settings.armor;
        let backups = self.config.settings.backups;
        let result = encrypt_document(&open.doc, &open.creds, armor)
            .and_then(|data| storage::save_atomic(&path, &data, backups))
            .and_then(|_| storage::acquire_lock(&path));
        match result {
            Ok(lock) => {
                let open = self.open.as_mut().unwrap();
                open.lock = lock;
                open.path = path.clone();
                open.dirty = false;
                self.config.add_recent(path);
                self.status = strings().saved.to_string();
            }
            Err(e) => self.status = format!("Save error: {e}"),
        }
    }

    fn execute_tree_actions(&mut self) {
        for ta in std::mem::take(&mut self.tree_actions) {
            let Some(open) = &mut self.open else {
                continue;
            };
            match ta {
                TreeAction::Select(id) => open.selected = id,
                TreeAction::NewChild(parent) => {
                    let id = open.doc.create_node(parent, "New note");
                    open.selected = id;
                    open.dirty = true;
                    self.renaming = Some(id);
                }
                TreeAction::Rename(id) => self.renaming = Some(id),
                TreeAction::Delete(id) => {
                    open.doc.remove_node(id);
                    if !open.doc.nodes.contains_key(&open.selected) {
                        open.selected = open.doc.meta.root;
                    }
                    open.dirty = true;
                }
                TreeAction::ToggleCollapse(id) => {
                    let c = open.doc.nodes[&id].collapsed;
                    open.doc.set_collapsed(id, !c);
                }
                TreeAction::Drop { dragged, target } => {
                    if open.doc.move_node(dragged, target, usize::MAX) {
                        open.dirty = true;
                    }
                }
                TreeAction::MoveToRoot(id) => {
                    let root = open.doc.meta.root;
                    if open.doc.move_node(id, root, usize::MAX) {
                        open.dirty = true;
                    }
                }
            }
        }
    }

    fn execute_actions(&mut self, ctx: &egui::Context) {
        for a in std::mem::take(&mut self.actions) {
            match a {
                Action::NewDocument => {
                    self.dialog = Some(Dialog::New(overlays::NewDocForm::default()));
                }
                Action::OpenDocument => {
                    if let Some(p) = rfd::FileDialog::new()
                        .add_filter("Partner", &["partner"])
                        .pick_file()
                    {
                        self.do_open_path(p);
                    }
                }
                Action::OpenRecent(p) => self.do_open_path(p),
                Action::Save => self.do_save(),
                Action::SaveAs => self.do_save_as(),
                Action::Lock => self.do_lock(),
                Action::Quit => {
                    if self.open.as_ref().map(|o| o.dirty).unwrap_or(false) {
                        self.do_save();
                    }
                    self.config.save();
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
                Action::Unlock => self.do_unlock(),
                Action::CancelUnlock => {
                    self.screen = Screen::Welcome;
                }
                Action::NewNote => {
                    if let Some(open) = &mut self.open {
                        let root = open.doc.meta.root;
                        let id = open.doc.create_node(root, "New note");
                        open.selected = id;
                        open.dirty = true;
                        self.renaming = Some(id);
                    }
                }
                Action::NewSubNote => {
                    if let Some(open) = &mut self.open {
                        let parent = open.selected;
                        let id = open.doc.create_node(parent, "New sub-note");
                        open.selected = id;
                        open.dirty = true;
                        self.renaming = Some(id);
                    }
                }
                Action::DeleteSelected => {
                    if let Some(open) = &mut self.open {
                        let id = open.selected;
                        if id != open.doc.meta.root {
                            open.doc.remove_node(id);
                            open.selected = open.doc.meta.root;
                            open.dirty = true;
                        }
                    }
                }
                Action::TogglePreview => self.show_preview = !self.show_preview,
                Action::ToggleSidebar => self.show_sidebar = !self.show_sidebar,
                Action::OpenSwitcher => {
                    if matches!(self.screen, Screen::Editor) {
                        self.palette = None;
                        self.switcher = Some(Switcher {
                            filter: String::new(),
                            selected: 0,
                            focus: true,
                        });
                    }
                }
                Action::OpenPalette => {
                    self.switcher = None;
                    self.palette = Some(Palette {
                        filter: String::new(),
                        selected: 0,
                        focus: true,
                    });
                }
                Action::OpenSettings => self.dialog = Some(Dialog::Settings),
                Action::OpenAbout => self.dialog = Some(Dialog::About),
                Action::PasteImage => {
                    if let Some(open) = &mut self.open {
                        let id = open.selected;
                        match editor::paste_image(&mut open.doc, id) {
                            Ok(_) => {
                                open.dirty = true;
                                self.status = "image embedded".to_string();
                            }
                            Err(e) => self.status = e,
                        }
                    }
                }
                Action::CheckUpdates => {
                    if !updater::is_flatpak() {
                        self.update_running = true;
                        self.last_update_check = Some(Instant::now());
                        let tx = self.update_tx.clone();
                        let v = VERSION.to_string();
                        thread::spawn(move || updater::check_for_update(&v, tx));
                    }
                }
                Action::UpdateDownload => {
                    if let UpdateState::Available(rel) = &self.update_state {
                        let rel = rel.clone();
                        self.update_running = true;
                        let tx = self.update_tx.clone();
                        thread::spawn(move || updater::download_update(&rel, tx));
                    }
                }
                Action::UpdateInstall => {
                    if let UpdateState::Downloaded { path, .. } = &self.update_state {
                        let path = path.clone();
                        if cfg!(windows) {
                            if updater::install_and_restart(&path).is_ok() {
                                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                            }
                        } else {
                            self.status = format!("Downloaded to {}", path.display());
                        }
                    }
                }
                Action::UpdateDismiss => self.update_state = UpdateState::Idle,
            }
        }
    }
}

impl eframe::App for PartnerApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.poll_activity(ui);
        self.handle_shortcuts(ui);
        self.drain_update_msgs();
        self.run_timers();
        match &mut self.screen {
            Screen::Welcome => self.ui_welcome(ui),
            Screen::Unlock { .. } => self.ui_unlock(ui),
            Screen::Editor => self.ui_editor(ui),
        }
        let ctx = ui.ctx().clone();
        self.render_overlays(&ctx);
        self.execute_tree_actions();
        self.execute_actions(&ctx);
        ctx.request_repaint_after(Duration::from_secs(1));
    }
}
