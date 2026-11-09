use crate::i18n::strings;
use eframe::egui::{self, RichText};
use partner_core::model::{Document, NodeId};

pub enum TreeAction {
    Select(NodeId),
    NewChild(NodeId),
    Rename(NodeId),
    Delete(NodeId),
    ToggleCollapse(NodeId),
    Drop { dragged: NodeId, target: NodeId },
    MoveToRoot(NodeId),
}

pub struct TreeCtx<'a> {
    pub doc: &'a mut Document,
    pub selected: &'a mut NodeId,
    pub renaming: &'a mut Option<NodeId>,
    pub search: &'a str,
    pub dirty: &'a mut bool,
    pub actions: &'a mut Vec<TreeAction>,
}

pub fn render_tree(ui: &mut egui::Ui, ctx: &mut TreeCtx) {
    ui.add_space(4.0);
    if !ctx.search.trim().is_empty() {
        let hits = ctx.doc.search(ctx.search);
        ui.label(RichText::new(format!("{} result(s)", hits.len())).weak());
        ui.add_space(2.0);
        for id in hits {
            let title = ctx.doc.nodes[&id].title.clone();
            if ui.selectable_label(*ctx.selected == id, &title).clicked() {
                ctx.actions.push(TreeAction::Select(id));
            }
        }
        return;
    }
    let root = ctx.doc.meta.root;
    let children = ctx.doc.nodes[&root].children.clone();
    for c in children {
        render_node(ui, ctx, c, 0);
    }
    let resp = ui.interact(
        ui.available_rect_before_wrap(),
        ui.scope_id().with("drop_root"),
        egui::Sense::hover(),
    );
    if let Some(p) = resp.dnd_release_payload::<NodeId>() {
        let dragged = *p;
        if dragged != root {
            ctx.actions.push(TreeAction::MoveToRoot(dragged));
        }
    }
}

fn render_node(ui: &mut egui::Ui, ctx: &mut TreeCtx, id: NodeId, depth: usize) {
    let Some(node) = ctx.doc.nodes.get(&id) else {
        return;
    };
    let has_children = !node.children.is_empty();
    let collapsed = node.collapsed;
    let title = node.title.clone();
    let is_sel = *ctx.selected == id;
    let is_renaming = *ctx.renaming == Some(id);
    ui.horizontal(|ui| {
        ui.add_space(depth as f32 * 14.0);
        if has_children {
            let arrow = if collapsed { "▸" } else { "▾" };
            if ui.small_button(arrow).clicked() {
                ctx.actions.push(TreeAction::ToggleCollapse(id));
            }
        } else {
            ui.add_space(16.0);
        }
        let label_text: &str = if title.is_empty() {
            "(untitled)"
        } else {
            &title
        };
        let resp = if is_renaming {
            let node = ctx.doc.nodes.get_mut(&id).unwrap();
            let out = egui::TextEdit::singleline(&mut node.title)
                .desired_width(140.0)
                .show(ui);
            let done = out.response.lost_focus() || ui.input(|i| i.key_pressed(egui::Key::Enter));
            if done {
                *ctx.renaming = None;
                *ctx.dirty = true;
            }
            out.response.response
        } else {
            let r = ui.selectable_label(is_sel, label_text);
            if r.clicked() {
                ctx.actions.push(TreeAction::Select(id));
            }
            r
        };
        resp.dnd_set_drag_payload(id);
        if resp.hovered() && resp.dnd_hover_payload::<NodeId>().is_some() {
            ui.painter().rect_stroke(
                resp.rect,
                2.0,
                ui.visuals().selection.stroke,
                egui::StrokeKind::Inside,
            );
        }
        if let Some(p) = resp.dnd_release_payload::<NodeId>() {
            let dragged = *p;
            if dragged != id {
                ctx.actions.push(TreeAction::Drop {
                    dragged,
                    target: id,
                });
            }
        }
        let s = strings();
        resp.context_menu(|ui| {
            if ui.button(s.new_subnote).clicked() {
                ctx.actions.push(TreeAction::NewChild(id));
                ui.close();
            }
            if ui.button(s.rename).clicked() {
                ctx.actions.push(TreeAction::Rename(id));
                ui.close();
            }
            if ui.button(s.delete).clicked() {
                ctx.actions.push(TreeAction::Delete(id));
                ui.close();
            }
        });
    });
    if has_children && !collapsed {
        let children = ctx.doc.nodes[&id].children.clone();
        for c in children {
            render_node(ui, ctx, c, depth + 1);
        }
    }
}
