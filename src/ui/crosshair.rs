use super::widgets::{
    action, card, check, draw_fit, form, ghost, heading, hint, icon_button, name_it, primary, row,
    segment, texture, thumb, toggle,
};
use super::{App, ListenTarget, Page, Semi, pal};
use crate::config::{self, Crosshair, MAX_GRID, MAX_IMAGE, MAX_NAME, MAX_SCALE, Mode};
use crate::{overlay, picture};
use eframe::egui::{
    self, Align2, Color32, CornerRadius, FontId, Rect, RichText, Sense, Slider, Stroke, StrokeKind,
    pos2, vec2,
};
use egui_phosphor::bold as icon;

fn slider(ui: &mut egui::Ui, label: &str, v: &mut u32, range: std::ops::RangeInclusive<u32>) {
    slider_when(ui, true, label, v, range);
}

/// A labelled slider that can be greyed out, for settings that only apply when another is on.
fn slider_when(
    ui: &mut egui::Ui,
    enabled: bool,
    label: &str,
    v: &mut u32,
    range: std::ops::RangeInclusive<u32>,
) {
    let text = ui.label(RichText::new(label).color(pal().muted));
    ui.add_enabled(enabled, Slider::new(v, range))
        .labelled_by(text.id);
    ui.end_row();
}

/// Six one-click colours for the usual picks, beside a colour button. Keeps the alpha.
fn swatches(ui: &mut egui::Ui, colour: &mut [u8; 4]) {
    const PICKS: [(&str, [u8; 3]); 6] = [
        ("绿色", [0, 255, 0]),
        ("青色", [0, 255, 255]),
        ("黄色", [255, 255, 0]),
        ("洋红", [255, 0, 255]),
        ("红色", [255, 0, 0]),
        ("白色", [255, 255, 255]),
    ];
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        for (name, rgb) in PICKS {
            let (rect, mut resp) = ui.allocate_exact_size(vec2(28.0, 28.0), Sense::click());
            let ring = rect.expand(2.0);
            // Held down, the chip sinks 1 px inside its ring, which stays put.
            let body = if resp.is_pointer_button_down_on() {
                rect.shrink(1.0)
            } else {
                rect
            };
            let p = ui.painter();
            p.rect_filled(body, 4, Color32::from_rgb(rgb[0], rgb[1], rgb[2]));
            p.rect_stroke(body, 4, Stroke::new(1.0, pal().edge), StrokeKind::Inside);
            if colour[..3] == rgb || resp.hovered() {
                p.rect_stroke(ring, 6, Stroke::new(2.0, pal().text), StrokeKind::Outside);
            }
            if resp.has_focus() {
                p.rect_stroke(ring, 6, ui.visuals().selection.stroke, StrokeKind::Outside);
            }
            if resp.clicked() {
                colour[..3].copy_from_slice(&rgb);
                resp.mark_changed();
            }
            name_it(resp, name)
                .on_hover_text(name)
                .on_hover_cursor(egui::CursorIcon::PointingHand);
        }
    });
}

fn mode_label(mode: Mode) -> &'static str {
    match mode {
        Mode::Lines => "线条",
        Mode::Pixels => "像素画布",
        Mode::Image => "图片",
    }
}

impl App {
    /// The Crosshair page's heading, and the preview when it isn't in its own column.
    pub(super) fn crosshair_header(&mut self, ui: &mut egui::Ui, with_preview: bool) {
        heading(ui, "准星", "修改会实时显示在屏幕准星上。");
        if with_preview {
            self.preview(ui, vec2(ui.available_width(), 150.0), false);
        }
    }

    fn crosshairs_ui(&mut self, ui: &mut egui::Ui) {
        let mut switch_to = None;
        let mut delete_at = None;
        let mut duplicate_cur = false;
        let mut add_new = false;
        let mut commit_rename = None;
        let mut hotkey_actions: Vec<(ListenTarget, Option<crate::config::Hotkey>)> = Vec::new();
        let can_delete = self.local.crosshairs.len() > 1;
        let active_idx = self.local.active_crosshair;

        card(ui, "", |ui| {
            egui::Sides::new().shrink_left().show(
                ui,
                |ui| super::widgets::caption(ui, "独立准星"),
                |ui| {
                    if primary(ui, true, &format!("{}  新建准星", icon::PLUS)).clicked() {
                        add_new = true;
                    }
                    if ghost(ui, &format!("{}  复制当前", icon::COPY)).clicked() {
                        duplicate_cur = true;
                    }
                },
            );
            hint(ui, "每个准星都有独立的样式、X/Y 坐标位置和快捷键；切换准星时会保留各自的位置。");
            ui.add_space(6.0);

            let count = self.local.crosshairs.len();
            for i in 0..count {
                let slot = self.local.crosshairs[i].clone();
                let is_active = i == active_idx;
                let target = ListenTarget::Crosshair(i);
                let listening = self.listening.as_ref() == Some(&target);
                let issue = self.hotkey_issue(&target, slot.hotkey);
                let t = texture(
                    &mut self.textures,
                    ui.ctx(),
                    &format!("slot:{i}"),
                    &slot.crosshair,
                );
                let (fill, stroke) = if is_active {
                    (pal().field, Stroke::new(1.5, pal().accent))
                } else {
                    (Color32::TRANSPARENT, Stroke::new(1.0, pal().border))
                };
                egui::Frame::new()
                    .fill(fill)
                    .stroke(stroke)
                    .corner_radius(8)
                    .inner_margin(egui::Margin::symmetric(12, 8))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        egui::Sides::new()
                            .shrink_left()
                            .truncate()
                            .height(40.0)
                            .spacing(10.0)
                            .show(
                                ui,
                                |ui| {
                                    thumb(ui, &t, 36.0);
                                    ui.vertical(|ui| {
                                        match &mut self.renaming_crosshair {
                                            Some((idx, text)) if *idx == i => {
                                                let r = ui.add(
                                                    egui::TextEdit::singleline(&mut *text)
                                                        .desired_width(130.0)
                                                        .char_limit(MAX_NAME),
                                                );
                                                if !r.has_focus() && !r.lost_focus() {
                                                    r.request_focus();
                                                }
                                                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                                                    self.renaming_crosshair = None;
                                                } else if r.lost_focus() {
                                                    commit_rename = Some((i, text.clone()));
                                                    self.renaming_crosshair = None;
                                                }
                                            }
                                            _ => {
                                                ui.horizontal(|ui| {
                                                    ui.spacing_mut().item_spacing.x = 4.0;
                                                    let label = ui.add(
                                                        egui::Label::new(
                                                            RichText::new(&slot.name).semi(),
                                                        )
                                                        .sense(Sense::click())
                                                        .truncate(),
                                                    );
                                                    if label
                                                        .on_hover_text("点击切换到该准星")
                                                        .clicked()
                                                    {
                                                        switch_to = Some(i);
                                                    }
                                                    if icon_button(
                                                        ui,
                                                        icon::PENCIL_SIMPLE,
                                                        format!("重命名 {}", slot.name),
                                                        "重命名",
                                                    )
                                                    .clicked()
                                                    {
                                                        self.renaming_crosshair =
                                                            Some((i, slot.name.clone()));
                                                    }
                                                });
                                            }
                                        }
                                        hint(
                                            ui,
                                            format!(
                                                "{}  ·  X: {}  Y: {}",
                                                mode_label(slot.crosshair.mode),
                                                slot.crosshair.offset_x,
                                                slot.crosshair.offset_y
                                            ),
                                        );
                                    });
                                },
                                |ui| {
                                    if can_delete {
                                        let armed = self.confirm_delete_crosshair == Some(i);
                                        let text = if armed {
                                            format!("{}  确认？", icon::TRASH)
                                        } else {
                                            icon::TRASH.to_string()
                                        };
                                        let btn = ui
                                            .scope(|ui| {
                                                let p = pal();
                                                let w = &mut ui.visuals_mut().widgets;
                                                w.inactive.fg_stroke.color =
                                                    if armed { p.danger } else { p.muted };
                                                w.hovered.fg_stroke.color = p.danger;
                                                w.active.fg_stroke.color = p.danger;
                                                ui.add(
                                                    egui::Button::new(RichText::new(text))
                                                        .frame_when_inactive(false),
                                                )
                                            })
                                            .inner;
                                        let tip = if armed {
                                            "再次点击以删除"
                                        } else {
                                            "删除此准星"
                                        };
                                        if name_it(btn, format!("删除 {}", slot.name))
                                            .on_hover_text(tip)
                                            .clicked()
                                        {
                                            if armed {
                                                delete_at = Some(i);
                                            } else {
                                                self.confirm_delete_crosshair = Some(i);
                                            }
                                        }
                                    }

                                    if slot.hotkey.vk != 0
                                        && ghost(ui, icon::X).on_hover_text("清除快捷键").clicked()
                                    {
                                        hotkey_actions
                                            .push((target.clone(), Some(Default::default())));
                                    }
                                    if let Some(why) = issue {
                                        ui.label(
                                            RichText::new(icon::WARNING)
                                                .small()
                                                .color(pal().yellow.1),
                                        )
                                        .on_hover_text(why);
                                    }
                                    let key_label = if listening {
                                        "请按键…".to_string()
                                    } else if slot.hotkey.vk == 0 {
                                        "绑定快捷键".to_string()
                                    } else {
                                        overlay::key_name(slot.hotkey)
                                    };
                                    let key_btn = egui::Button::new(
                                        RichText::new(key_label).small().color(if listening {
                                            pal().accent
                                        } else {
                                            pal().text
                                        }),
                                    );
                                    if name_it(
                                        ui.add(key_btn),
                                        format!("设置 {} 的快捷键", slot.name),
                                    )
                                    .on_hover_text("点击后按下快捷键即可绑定（再次按下切换显示/隐藏）")
                                    .clicked()
                                    {
                                        hotkey_actions.push((target, None));
                                    }

                                    if is_active {
                                        ui.label(
                                            RichText::new(format!("{}  当前", icon::CHECK))
                                                .small()
                                                .color(pal().green.1),
                                        );
                                    } else if name_it(
                                        ui.add(egui::Button::new("切换")),
                                        format!("切换到 {}", slot.name),
                                    )
                                    .clicked()
                                    {
                                        switch_to = Some(i);
                                    }
                                },
                            );
                    });
                if i + 1 < count {
                    ui.add_space(6.0);
                }
            }
        });

        for (target, set_val) in hotkey_actions {
            match set_val {
                Some(hk) => self.set_hotkey(&target, hk),
                None => self.listening = Some(target),
            }
        }
        if let Some((idx, new_name)) = commit_rename {
            let _ = self.local.rename_crosshair(idx, &new_name);
        }
        if add_new {
            let mode = self.local.active_crosshair().mode;
            self.local.add_crosshair(mode);
            self.undo.clear();
            self.last_cell = None;
            self.confirm_delete_crosshair = None;
            self.renaming_crosshair = None;
        } else if duplicate_cur {
            self.local.duplicate_crosshair(active_idx);
            self.undo.clear();
            self.last_cell = None;
            self.confirm_delete_crosshair = None;
            self.renaming_crosshair = None;
        } else if let Some(idx) = delete_at {
            self.confirm_delete_crosshair = None;
            self.renaming_crosshair = None;
            if self.local.delete_crosshair(idx) {
                self.undo.clear();
                self.last_cell = None;
            }
        } else if let Some(idx) = switch_to
            && idx != self.local.active_crosshair
            && self.local.set_active_crosshair(idx)
        {
            self.undo.clear();
            self.last_cell = None;
            self.confirm_delete_crosshair = None;
        }
    }

    pub(super) fn crosshair_body(&mut self, ui: &mut egui::Ui) {
        self.crosshairs_ui(ui);

        let c = self.local.active_crosshair_mut();
        ui.horizontal(|ui| {
            for (m, ic, name) in [
                (Mode::Lines, icon::CROSSHAIR_SIMPLE, "线条"),
                (Mode::Pixels, icon::PAINT_BRUSH, "像素画布"),
                (Mode::Image, icon::IMAGE, "图片"),
            ] {
                if segment(ui, format!("{ic}  {name}"), c.mode == m, vec2(130.0, 34.0)).clicked() {
                    c.mode = m;
                }
            }
        });
        ui.add_space(14.0);

        match self.local.active_crosshair().mode {
            Mode::Lines => lines_ui(ui, self.local.active_crosshair_mut()),
            Mode::Pixels => self.pixels_ui(ui),
            Mode::Image => self.image_ui(ui),
        }

        let active_name = self
            .local
            .crosshairs
            .get(self.local.active_crosshair)
            .map_or_else(|| "当前准星".to_string(), |s| s.name.clone());
        let c = self.local.active_crosshair_mut();
        card(ui, "位置", |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("偏移").color(pal().muted));
                ui.add(
                    egui::DragValue::new(&mut c.offset_x)
                        .range(-3000..=3000)
                        .prefix("X  "),
                );
                ui.add(
                    egui::DragValue::new(&mut c.offset_y)
                        .range(-3000..=3000)
                        .prefix("Y  "),
                );
                if ghost(ui, &format!("{}  重置", icon::ARROW_COUNTER_CLOCKWISE)).clicked() {
                    (c.offset_x, c.offset_y) = (0, 0);
                }
            });
            hint(ui, format!("仅调整「{active_name}」的屏幕位置偏移，不影响其他独立准星。"));
        });
    }

    /// Crosshair over a dark and a light background, so you can judge contrast on both.
    /// Side by side normally, stacked when it has its own column on a wide window.
    pub(super) fn preview(&mut self, ui: &mut egui::Ui, size: egui::Vec2, stacked: bool) {
        let active = self.local.active_crosshair().clone();
        let t = texture(&mut self.textures, ui.ctx(), "current", &active);
        let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
        let p = ui.painter_at(rect);
        let round = |nw, ne, sw, se| CornerRadius { nw, ne, sw, se };
        let (dark, light, dark_round, light_round) = if stacked {
            let mid = rect.center().y.round();
            (
                Rect::from_min_max(rect.min, pos2(rect.max.x, mid)),
                Rect::from_min_max(pos2(rect.min.x, mid), rect.max),
                round(10, 10, 0, 0),
                round(0, 0, 10, 10),
            )
        } else {
            let mid = rect.center().x.round();
            (
                Rect::from_min_max(rect.min, pos2(mid, rect.max.y)),
                Rect::from_min_max(pos2(mid, rect.min.y), rect.max),
                round(10, 0, 10, 0),
                round(0, 10, 0, 10),
            )
        };
        p.rect_filled(dark, dark_round, Color32::from_rgb(8, 8, 8));
        p.rect_filled(light, light_round, Color32::from_rgb(196, 192, 184));
        p.rect_stroke(rect, 10, Stroke::new(1.0, pal().border), StrokeKind::Inside);
        let fit = dark.width().min(dark.height()) - 40.0;
        let mut zoom = 1.0;
        for half in [dark, light] {
            zoom = draw_fit(&p, &t, half.center(), fit);
        }
        let size = t.handle.size_vec2();
        // Drawn on the light half, so the ink is dark enough for 4.5:1 against it.
        p.text(
            rect.right_bottom() - vec2(12.0, 10.0),
            Align2::RIGHT_BOTTOM,
            format!("{}×{} 像素  ·  放大 {zoom:.0}×", size.x, size.y),
            FontId::monospace(12.0),
            Color32::from_rgb(56, 54, 52),
        );
        ui.add_space(16.0);
    }

    fn image_ui(&mut self, ui: &mut egui::Ui) {
        let mut choose = false;
        let c = self.local.active_crosshair_mut();
        card(ui, "图片", |ui| {
            ui.horizontal(|ui| {
                choose = primary(ui, true, &format!("{}  选择图片…", icon::FOLDER_OPEN)).clicked();
                let current = if c.image.is_empty() {
                    "尚未选择图片".to_string()
                } else {
                    "已加载图片".to_string()
                };
                ui.label(RichText::new(current).color(pal().muted));
            });
            hint(ui, "或直接将图片拖入此窗口，推荐使用透明背景的 PNG 图片。");
            if let Some(e) = &self.image_error {
                ui.label(
                    RichText::new(format!("{}  {e}", icon::WARNING))
                        .small()
                        .color(pal().yellow.1),
                );
            }
            ui.add_space(4.0);
            form(ui, "image_opts", |ui| {
                row(ui, "大小", |ui| {
                    ui.add(Slider::new(&mut c.image_size, 4..=MAX_IMAGE).suffix(" 像素"));
                });
                row(ui, "不透明度", |ui| {
                    ui.add(Slider::new(&mut c.image_opacity, 5..=100).suffix("%"));
                });
            });
        });
        if choose && let Some(path) = picture::pick(overlay::settings_window()) {
            self.use_image(&path);
        }
    }

    /// Import a picture (from the dialog or dropped on the window) and switch to it.
    pub(super) fn use_image(&mut self, path: &std::path::Path) {
        if picture::load(path, 16).is_none() {
            self.image_error = Some("Windows 无法打开该图片文件。".into());
            return;
        }
        match config::import_image(path) {
            Ok(name) => {
                let c = self.local.active_crosshair_mut();
                c.mode = Mode::Image;
                c.image = name;
                self.page = Page::Crosshair;
                self.image_error = None;
            }
            Err(e) => self.image_error = Some(format!("无法导入图片：{e}")),
        }
    }

    fn pixels_ui(&mut self, ui: &mut egui::Ui) {
        let mut brush = self.brush;
        let mut mirror_x = self.mirror_x;
        let mut mirror_y = self.mirror_y;
        let undo = &mut self.undo;
        let last_cell = &mut self.last_cell;
        let c = self.local.active_crosshair_mut();
        card(ui, "画布", |ui| {
            form(ui, "canvas_opts", |ui| {
                let mut n = c.grid;
                row(ui, "尺寸", |ui| {
                    let r = ui.add(Slider::new(&mut n, 3..=MAX_GRID).suffix(" 格"));
                    // Shrinking crops the drawing, so make it undoable (once per drag).
                    if r.drag_started() || (r.changed() && !r.dragged()) {
                        undo.push((c.grid, c.pixels.clone()));
                    }
                    if r.changed() {
                        c.resize_grid(n);
                    }
                });
                slider(ui, "像素缩放", &mut c.scale, 1..=MAX_SCALE);
                row(ui, "画笔", |ui| {
                    ui.horizontal(|ui| {
                        name_it(ui.color_edit_button_srgba_unmultiplied(&mut brush), "画笔颜色");
                        swatches(ui, &mut brush);
                    });
                });
                // The mirror toggles get their own line: next to the swatches they'd run off
                // a narrow window.
                row(ui, "", |ui| {
                    ui.horizontal(|ui| {
                        toggle(ui, &mut mirror_x, &format!("{} 水平镜像", icon::FLIP_HORIZONTAL));
                        toggle(ui, &mut mirror_y, &format!("{} 垂直镜像", icon::FLIP_VERTICAL));
                    });
                });
                row(ui, "", |ui| {
                    ui.horizontal(|ui| {
                        let undo_btn = action(
                            ui,
                            !undo.is_empty(),
                            RichText::new(format!("{}  撤销", icon::ARROW_U_UP_LEFT)),
                        )
                        .on_hover_text("Ctrl+Z");
                        let ctrl_z =
                            egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, egui::Key::Z);
                        let ctrl_z = ui.input_mut(|i| i.consume_shortcut(&ctrl_z));
                        if (undo_btn.clicked() || ctrl_z)
                            && let Some((grid, px)) = undo.pop()
                        {
                            (c.grid, c.pixels) = (grid, px);
                        }
                        if ghost(ui, &format!("{}  清空", icon::TRASH)).clicked() {
                            undo.push((c.grid, c.pixels.clone()));
                            c.pixels.fill([0; 4]);
                        }
                    });
                });
            });
            hint(
                ui,
                format!(
                    "{} 左键绘制    {} 右键擦除    {} 中键取色",
                    icon::PAINT_BRUSH,
                    icon::ERASER,
                    icon::EYEDROPPER
                ),
            );
            ui.add_space(8.0);

            if undo.len() > 50 {
                undo.remove(0);
            }
            let n = c.grid as usize;
            let cell = (480.0 / n as f32).floor().clamp(5.0, 26.0);
            let (resp, p) = ui.allocate_painter(
                vec2(cell * n as f32, cell * n as f32),
                Sense::click_and_drag(),
            );
            let r = resp.rect;
            p.rect_filled(r, 0, Color32::from_rgb(10, 10, 10));
            let cell_rect = |x: usize, y: usize| {
                Rect::from_min_size(
                    pos2(r.min.x + x as f32 * cell, r.min.y + y as f32 * cell),
                    vec2(cell, cell),
                )
            };
            for (i, px) in c.pixels.iter().enumerate() {
                if px[3] > 0 {
                    let [r, g, b, a] = *px;
                    p.rect_filled(
                        cell_rect(i % n, i / n),
                        0.0,
                        Color32::from_rgba_unmultiplied(r, g, b, a),
                    );
                }
            }
            let grid = Stroke::new(1.0, Color32::from_rgb(30, 30, 29));
            for i in 0..=n {
                let o = i as f32 * cell;
                p.line_segment(
                    [pos2(r.min.x + o, r.min.y), pos2(r.min.x + o, r.max.y)],
                    grid,
                );
                p.line_segment(
                    [pos2(r.min.x, r.min.y + o), pos2(r.max.x, r.min.y + o)],
                    grid,
                );
            }
            // Mark the screen centre.
            let centre = Stroke::new(1.0, Color32::from_rgb(200, 90, 85));
            if n % 2 == 1 {
                p.rect_stroke(cell_rect(n / 2, n / 2), 0.0, centre, StrokeKind::Inside);
            } else {
                let m = r.center();
                p.line_segment([pos2(m.x, r.min.y), pos2(m.x, r.max.y)], centre);
                p.line_segment([pos2(r.min.x, m.y), pos2(r.max.x, m.y)], centre);
            }
            if let Some(h) = resp.hover_pos() {
                let (x, y) = (
                    ((h.x - r.min.x) / cell) as usize,
                    ((h.y - r.min.y) / cell) as usize,
                );
                if x < n && y < n {
                    p.rect_stroke(
                        cell_rect(x, y),
                        0.0,
                        Stroke::new(1.0, pal().accent),
                        StrokeKind::Inside,
                    );
                }
            }

            if let Some(pos) = resp.interact_pointer_pos() {
                let cur = (
                    ((pos.x - r.min.x) / cell).floor() as isize,
                    ((pos.y - r.min.y) / cell).floor() as isize,
                );
                let inside = |(x, y): (isize, isize)| {
                    (0..n as isize).contains(&x) && (0..n as isize).contains(&y)
                };
                let (erase, pick) =
                    ui.input(|i| (i.pointer.secondary_down(), i.pointer.middle_down()));
                if pick {
                    if inside(cur) {
                        brush = c.pixels[cur.1 as usize * n + cur.0 as usize];
                    }
                } else {
                    let from = match *last_cell {
                        Some(prev) => prev,
                        None => {
                            undo.push((c.grid, c.pixels.clone())); // one undo step per stroke
                            cur
                        }
                    };
                    *last_cell = Some(cur);
                    let col = if erase { [0; 4] } else { brush };
                    // Walk from the previous cell to this one so quick strokes have no gaps.
                    let steps = (cur.0 - from.0).abs().max((cur.1 - from.1).abs()).max(1);
                    for step in 0..=steps {
                        let t = step as f32 / steps as f32;
                        let p = (
                            (from.0 as f32 + (cur.0 - from.0) as f32 * t).round() as isize,
                            (from.1 as f32 + (cur.1 - from.1) as f32 * t).round() as isize,
                        );
                        if !inside(p) {
                            continue;
                        }
                        let (x, y) = (p.0 as usize, p.1 as usize);
                        let xs = if mirror_x { [x, n - 1 - x] } else { [x, x] };
                        let ys = if mirror_y { [y, n - 1 - y] } else { [y, y] };
                        for yy in ys {
                            for xx in xs {
                                c.pixels[yy * n + xx] = col;
                            }
                        }
                    }
                }
            } else {
                *last_cell = None;
            }
        });
        self.brush = brush;
        self.mirror_x = mirror_x;
        self.mirror_y = mirror_y;
    }
}

fn lines_ui(ui: &mut egui::Ui, c: &mut Crosshair) {
    card(ui, "颜色", |ui| {
        // Wraps the swatches onto a second line on a narrow window instead of running off the card.
        ui.horizontal_wrapped(|ui| {
            let fill = ui.label(RichText::new("填充").color(pal().muted));
            ui.color_edit_button_srgba_unmultiplied(&mut c.color)
                .labelled_by(fill.id);
            ui.add_space(8.0);
            swatches(ui, &mut c.color);
            ui.add_space(20.0);
            // Label and swatch wrap as a pair, so the swatch is never stranded on its own line.
            ui.horizontal(|ui| {
                let outline = ui.label(RichText::new("描边").color(pal().muted));
                ui.color_edit_button_srgba_unmultiplied(&mut c.outline_color)
                    .labelled_by(outline.id);
            });
        });
    });
    card(ui, "线条", |ui| {
        form(ui, "lines", |ui| {
            slider(ui, "长度", &mut c.length, 0..=50);
            slider(ui, "粗细", &mut c.thickness, 1..=20);
            slider(ui, "间距", &mut c.gap, 0..=50);
            slider(ui, "描边宽度", &mut c.outline, 0..=5);
            row(ui, "", |ui| {
                check(ui, &mut c.t_style, "T 型准星（无上边线）");
            });
        });
    });
    card(ui, "中心点与圆环", |ui| {
        form(ui, "extras", |ui| {
            row(ui, "", |ui| {
                check(ui, &mut c.dot, "中心圆点");
            });
            slider_when(ui, c.dot, "圆点大小", &mut c.dot_size, 1..=20);
            row(ui, "", |ui| {
                check(ui, &mut c.circle, "圆环");
            });
            slider_when(ui, c.circle, "半径", &mut c.circle_radius, 1..=100);
            slider_when(ui, c.circle, "圆环宽度", &mut c.circle_thickness, 1..=10);
        });
    });
}
