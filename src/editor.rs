use eframe::egui::{self, RichText};
use partner_core::model::{Document, NodeId, BLOB_URI_SCHEME};
use std::collections::HashMap;

pub fn cursor_byte_range(
    text: &str,
    cr: Option<egui::text::CCursorRange>,
) -> Option<(usize, usize)> {
    let cr = cr?;
    let a = cr.primary.index.0;
    let b = cr.secondary.index.0;
    let (lo, hi) = (a.min(b), a.max(b));
    let lo_b = text
        .char_indices()
        .nth(lo)
        .map(|(i, _)| i)
        .unwrap_or(text.len());
    let hi_b = text
        .char_indices()
        .nth(hi)
        .map(|(i, _)| i)
        .unwrap_or(text.len());
    Some((lo_b, hi_b))
}

pub fn insert_snippet(
    doc: &mut Document,
    id: NodeId,
    cursor: Option<egui::text::CCursorRange>,
    snippet: &str,
) {
    let Some(node) = doc.nodes.get_mut(&id) else {
        return;
    };
    let body = &mut node.body;
    match cursor_byte_range(body, cursor) {
        Some((lo, hi)) if lo != hi => {
            let selected = body[lo..hi].to_string();
            let wrapped = format!("{snippet}{selected}{snippet}");
            body.replace_range(lo..hi, &wrapped);
        }
        Some((pos, _)) => {
            body.insert_str(pos, snippet);
        }
        None => body.push_str(snippet),
    }
}

pub fn clipboard_image_to_png() -> Option<Vec<u8>> {
    let mut cb = arboard::Clipboard::new().ok()?;
    let img = cb.get_image().ok()?;
    let rgba = img.bytes.to_vec();
    let raw = image::RgbaImage::from_raw(img.width as u32, img.height as u32, rgba)?;
    let dyn_img = image::DynamicImage::ImageRgba8(raw);
    let mut cursor = std::io::Cursor::new(Vec::new());
    dyn_img
        .write_to(&mut cursor, image::ImageFormat::Png)
        .ok()?;
    Some(cursor.into_inner())
}

pub fn paste_image(doc: &mut Document, id: NodeId) -> Result<String, String> {
    let png = clipboard_image_to_png().ok_or("no image in clipboard")?;
    let blob_id = doc.add_blob("image/png", png);
    let node = doc.nodes.get_mut(&id).ok_or("no such note")?;
    if !node.body.is_empty() && !node.body.ends_with('\n') {
        node.body.push('\n');
    }
    node.body
        .push_str(&format!("![pasted image]({BLOB_URI_SCHEME}{blob_id})\n"));
    Ok(blob_id)
}

pub fn render_editor(
    ui: &mut egui::Ui,
    doc: &mut Document,
    id: NodeId,
    dirty: &mut bool,
    last_cursor: &mut Option<egui::text::CCursorRange>,
) {
    {
        let node = doc.nodes.get_mut(&id).unwrap();
        let out = egui::TextEdit::singleline(&mut node.title)
            .font(egui::TextStyle::Heading)
            .desired_width(f32::INFINITY)
            .show(ui);
        if out.response.changed() {
            *dirty = true;
        }
    }
    ui.separator();
    ui.horizontal(|ui| {
        let mut ins = |snippet: &str| {
            insert_snippet(doc, id, *last_cursor, snippet);
            *dirty = true;
        };
        if ui.button(RichText::new("B").strong()).clicked() {
            ins("**bold**");
        }
        if ui.button(RichText::new("I").italics()).clicked() {
            ins("*italic*");
        }
        if ui.button(RichText::new("S").strikethrough()).clicked() {
            ins("~~strike~~");
        }
        if ui.button(RichText::new("` code `").code()).clicked() {
            ins("`code`");
        }
        if ui.button("H1").clicked() {
            ins("# ");
        }
        if ui.button("</>").clicked() {
            ins("```\n\n```\n");
        }
        if ui.button("🔗").clicked() {
            ins("[text](https://)");
        }
    });
    ui.separator();
    let out = {
        let node = doc.nodes.get_mut(&id).unwrap();
        let body = &mut node.body;
        egui::TextEdit::multiline(body)
            .desired_width(f32::INFINITY)
            .desired_rows(24)
            .font(egui::TextStyle::Monospace)
            .show(ui)
    };
    *last_cursor = out.cursor_range;
    if out.response.changed() {
        *dirty = true;
    }
}

#[derive(Clone, Copy, Default)]
struct Style {
    bold: bool,
    italic: bool,
    strike: bool,
    code: bool,
    link: bool,
}

enum LineKind {
    Normal,
    Heading(u8),
}

pub fn render_markdown(
    ui: &mut egui::Ui,
    text: &str,
    doc: &Document,
    textures: &mut HashMap<String, egui::TextureHandle>,
) {
    use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    let parser = Parser::new_ext(text, options);

    let mut style = Style::default();
    let mut inline: Vec<(String, Style)> = Vec::new();
    let mut line = LineKind::Normal;
    let mut list_stack: Vec<Option<u64>> = Vec::new();
    let mut list_counters: Vec<u64> = Vec::new();
    let mut quote_depth: u32 = 0;
    let mut code_block: Option<(String, String)> = None;
    let mut table_rows: Vec<Vec<String>> = Vec::new();
    let mut table_row: Vec<String> = Vec::new();
    let mut table_cell = String::new();
    let mut in_cell = false;
    let mut table_id: u64 = 0;

    macro_rules! flush_inline {
        () => {
            if !inline.is_empty() {
                let kind = std::mem::replace(&mut line, LineKind::Normal);
                let q = quote_depth;
                ui.horizontal_wrapped(|ui| {
                    if q > 0 {
                        ui.label(RichText::new("❯".repeat(q as usize)).weak());
                    }
                    match kind {
                        LineKind::Heading(h) => {
                            let size = match h {
                                1 => 26.0,
                                2 => 22.0,
                                3 => 18.0,
                                _ => 16.0,
                            };
                            let text: String = inline.drain(..).map(|(t, _)| t).collect();
                            ui.add(
                                egui::Label::new(RichText::new(text).size(size).strong()).wrap(),
                            );
                        }
                        LineKind::Normal => {
                            for (t, s) in inline.drain(..) {
                                let mut rt = RichText::new(t);
                                if s.bold {
                                    rt = rt.strong();
                                }
                                if s.italic {
                                    rt = rt.italics();
                                }
                                if s.strike {
                                    rt = rt.strikethrough();
                                }
                                if s.code {
                                    rt = rt.code();
                                }
                                if s.link {
                                    rt = rt.color(ui.visuals().hyperlink_color).underline();
                                }
                                ui.label(rt);
                            }
                        }
                    }
                });
            }
        };
    }

    for event in parser {
        match event {
            Event::Start(tag) => match tag {
                Tag::Paragraph => {
                    flush_inline!();
                }
                Tag::Heading { level, .. } => {
                    flush_inline!();
                    line = LineKind::Heading(match level {
                        HeadingLevel::H1 => 1,
                        HeadingLevel::H2 => 2,
                        HeadingLevel::H3 => 3,
                        _ => 4,
                    });
                }
                Tag::Emphasis => style.italic = true,
                Tag::Strong => style.bold = true,
                Tag::Strikethrough => style.strike = true,
                Tag::Link { .. } => style.link = true,
                Tag::Image { dest_url, .. } => {
                    flush_inline!();
                    if let Some(id) = dest_url.strip_prefix(BLOB_URI_SCHEME) {
                        render_blob_image(ui, doc, textures, id);
                    } else {
                        ui.label(RichText::new(format!("[image: {dest_url}]")).weak());
                    }
                }
                Tag::CodeBlock(kind) => {
                    flush_inline!();
                    let lang = match kind {
                        CodeBlockKind::Fenced(l) => l.to_string(),
                        CodeBlockKind::Indented => String::new(),
                    };
                    code_block = Some((lang, String::new()));
                }
                Tag::List(start) => {
                    flush_inline!();
                    list_stack.push(start);
                    list_counters.push(start.unwrap_or(0));
                }
                Tag::Item => {
                    flush_inline!();
                    if let Some(c) = list_counters.last_mut() {
                        *c += 1;
                        let n = *c;
                        let ordered = list_stack.last().copied().flatten().is_some();
                        let prefix = if ordered {
                            format!("{n}. ")
                        } else {
                            "• ".to_string()
                        };
                        inline.push((prefix, Style::default()));
                    }
                }
                Tag::BlockQuote(_) => {
                    flush_inline!();
                    quote_depth += 1;
                }
                Tag::Table(_) => {
                    flush_inline!();
                    table_rows.clear();
                    table_id += 1;
                }
                Tag::TableRow => table_row.clear(),
                Tag::TableCell => {
                    in_cell = true;
                    table_cell.clear();
                }
                _ => {}
            },
            Event::End(tag) => match tag {
                TagEnd::Paragraph => flush_inline!(),
                TagEnd::Heading(_) => flush_inline!(),
                TagEnd::Emphasis => style.italic = false,
                TagEnd::Strong => style.bold = false,
                TagEnd::Strikethrough => style.strike = false,
                TagEnd::Link => style.link = false,
                TagEnd::Image => {}
                TagEnd::CodeBlock => {
                    if let Some((lang, buf)) = code_block.take() {
                        let label = if lang.is_empty() {
                            "code".to_string()
                        } else {
                            lang
                        };
                        egui::Frame::window(ui.style()).show(ui, |ui| {
                            ui.label(RichText::new(label).small().weak());
                            ui.separator();
                            ui.label(RichText::new(buf).family(egui::FontFamily::Monospace));
                        });
                        ui.add_space(4.0);
                    }
                }
                TagEnd::List(_) => {
                    flush_inline!();
                    list_stack.pop();
                    list_counters.pop();
                }
                TagEnd::Item => flush_inline!(),
                TagEnd::BlockQuote(_) => {
                    flush_inline!();
                    quote_depth = quote_depth.saturating_sub(1);
                }
                TagEnd::Table => {
                    egui::Grid::new(format!("tbl-{table_id}"))
                        .striped(true)
                        .show(ui, |ui| {
                            for row in &table_rows {
                                for cell in row {
                                    ui.label(cell);
                                }
                                ui.end_row();
                            }
                        });
                    ui.add_space(4.0);
                }
                TagEnd::TableRow => {
                    table_rows.push(std::mem::take(&mut table_row));
                }
                TagEnd::TableCell => {
                    table_row.push(std::mem::take(&mut table_cell));
                    in_cell = false;
                }
                _ => {}
            },
            Event::Text(t) => {
                if in_cell {
                    table_cell.push_str(&t);
                } else if let Some((_, buf)) = &mut code_block {
                    buf.push_str(&t);
                } else {
                    inline.push((t.to_string(), style));
                }
            }
            Event::Code(t) => inline.push((
                t.to_string(),
                Style {
                    code: true,
                    ..style
                },
            )),
            Event::SoftBreak | Event::HardBreak => inline.push(("\n".to_string(), style)),
            Event::Rule => {
                flush_inline!();
                ui.separator();
            }
            Event::TaskListMarker(checked) => {
                inline.push((if checked { "☑ " } else { "☐ " }.to_string(), style));
            }
            _ => {}
        }
    }
    flush_inline!();
}

fn render_blob_image(
    ui: &mut egui::Ui,
    doc: &Document,
    textures: &mut HashMap<String, egui::TextureHandle>,
    blob_id: &str,
) {
    if let Some(tex) = textures.get(blob_id) {
        let size = tex.size_vec2();
        let max_w = 560.0f32.min(ui.available_width() - 20.0);
        let scale = (max_w / size.x).min(1.0);
        ui.image((tex.id(), size * scale));
        return;
    }
    let Some(blob) = doc.blobs.get(blob_id) else {
        ui.label(RichText::new("⚠ missing image").weak());
        return;
    };
    match image::load_from_memory(&blob.data) {
        Ok(img) => {
            let rgba = img.to_rgba8();
            let size = [rgba.width() as usize, rgba.height() as usize];
            let color = egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
            let tex =
                ui.ctx()
                    .load_texture(blob_id.to_string(), color, egui::TextureOptions::LINEAR);
            let max_w = 560.0f32.min(ui.available_width() - 20.0);
            let scale = (max_w / size[0] as f32).min(1.0);
            let sz = egui::vec2(size[0] as f32 * scale, size[1] as f32 * scale);
            ui.image((tex.id(), sz));
            textures.insert(blob_id.to_string(), tex);
        }
        Err(_) => {
            ui.label(RichText::new("⚠ undecodable image").weak());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::text::{CCursor, CCursorRange, CharIndex};
    use partner_core::model::Document;

    fn cr(a: usize, b: usize) -> Option<CCursorRange> {
        Some(CCursorRange {
            primary: CCursor {
                index: CharIndex(a),
                prefer_next_row: false,
            },
            secondary: CCursor {
                index: CharIndex(b),
                prefer_next_row: false,
            },
            h_pos: None,
        })
    }

    #[test]
    fn cursor_byte_range_handles_unicode() {
        let text = "héllo wörld";
        assert_eq!(cursor_byte_range(text, cr(0, 0)), Some((0, 0)));
        assert_eq!(cursor_byte_range(text, cr(2, 5)), Some((3, 6)));
        assert_eq!(cursor_byte_range(text, None), None);
    }

    #[test]
    fn insert_snippet_wraps_selection() {
        let mut doc = Document::new("t");
        let id = doc.create_node(0, "n");
        doc.set_body(id, "hello world");
        insert_snippet(&mut doc, id, cr(0, 5), "**");
        assert_eq!(doc.nodes[&id].body, "**hello** world");
    }

    #[test]
    fn insert_snippet_at_cursor() {
        let mut doc = Document::new("t");
        let id = doc.create_node(0, "n");
        doc.set_body(id, "ab");
        insert_snippet(&mut doc, id, cr(1, 1), "X");
        assert_eq!(doc.nodes[&id].body, "aXb");
    }
}
