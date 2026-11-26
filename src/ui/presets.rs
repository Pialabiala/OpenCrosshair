use super::widgets::{
    caption, card, empty_state, ghost, heading, hint, icon_button, name_it, primary, texture, thumb,
};
use super::{App, ListenTarget, Semi, pal};
use crate::config::{Crosshair, Hotkey, MAX_NAME, Mode};
use crate::{overlay, picture, share};
use eframe::egui::{self, Align2, FontId, RichText, Sense, Stroke, vec2};
use egui_phosphor::bold as icon;
use std::collections::BTreeMap;

impl App {
    pub(super) fn presets_page(&mut self, ui: &mut egui::Ui) {
        heading(
            ui,
            "预设",
            "保存准星配置并在它们之间快速切换，每个游戏也可指定专属预设。",
        );
        card(ui, "保存当前准星", |ui| {
            ui.horizontal(|ui| {
                let active = self.local.active_crosshair().clone();
                let t = texture(&mut self.textures, ui.ctx(), "current", &active);
                thumb(ui, &t, 38.0);
                ui.add(
                    egui::TextEdit::singleline(&mut self.preset_name)
                        .hint_text("预设名称")
                        .desired_width(240.0)
                        .char_limit(MAX_NAME),
                );
                let name = self.preset_name.trim().to_string();
                let label = if self.local.presets.contains_key(&name) {
                    "覆盖"
                } else {
                    "保存"
                };
                let text = format!("{}  {label}", icon::FLOPPY_DISK);
                let clicked = if name.is_empty() {
                    // egui fades disabled buttons, which left this gray on gray. Drawn flat in the
                    // secondary text colour instead, so it stays readable; clicks do nothing.
                    let p = pal();
                    let r = ui.add(
                        egui::Button::new(RichText::new(&text).color(p.muted).semi())
                            .fill(p.field)
                            .stroke(Stroke::new(1.0, p.border)),
                    );
                    r.widget_info(|| {
                        egui::WidgetInfo::labeled(egui::WidgetType::Button, false, label)
                    });
                    false
                } else {
                    primary(ui, true, &text).clicked()
                };
                if clicked {
                    let verb = if self.local.presets.contains_key(&name) {
                        "已更新"
                    } else {
                        "已保存"
                    };
                    self.preset_status = Some(format!("{verb}「{name}」。"));
                    self.local.presets.insert(name, active);
                }
            });
        });

        let mut load = None;
        let mut delete = None;
        let mut export = None;
        let mut export_all = false;
        let mut import = false;
        let mut commit_rename = None;
        let mut hotkey_actions: Vec<(ListenTarget, Option<Hotkey>)> = Vec::new();
        card(ui, "", |ui| {
            // Shrink left, so the buttons keep their room and the caption gives way when narrow.
            egui::Sides::new().shrink_left().show(
                ui,
                |ui| caption(ui, "已保存"),
                |ui| {
                    // Right to left, so Import goes first to sit to the right of Export all.
                    if ghost(ui, &format!("{}  导入", icon::DOWNLOAD_SIMPLE)).clicked() {
                        import = true;
                    }
                    if !self.local.presets.is_empty()
                        && ghost(ui, &format!("{}  全部导出", icon::EXPORT)).clicked()
                    {
                        export_all = true;
                    }
                },
            );
            ui.add_space(6.0);
            if let Some(s) = &self.preset_status {
                hint(ui, s);
            }
            ui.add_space(4.0);
            if self.local.presets.is_empty() {
                empty_state(
                    ui,
                    icon::BOOKMARKS_SIMPLE,
                    "暂无保存的预设",
                    "在上方保存当前准星即可作为预设，方便在各游戏间切换，或导入他人分享的预设文件。",
                );
            }
            for (name, c) in &self.local.presets {
                let t = texture(&mut self.textures, ui.ctx(), &format!("preset:{name}"), c);
                let hk = self
                    .local
                    .preset_hotkeys
                    .get(name)
                    .copied()
                    .unwrap_or_default();
                let target = ListenTarget::Preset(name.clone());
                let listening = self.listening.as_ref() == Some(&target);
                let issue = self.hotkey_issue(&target, hk);
                // Sides, not a Grid: a long name would widen its column and push the buttons off.
                egui::Sides::new()
                    .shrink_left()
                    .truncate()
                    .height(56.0) // the thumbnail's height, so the rest centres on it
                    .spacing(14.0) // room for the pencil icon after a cut-off name
                    .show(
                        ui,
                        |ui| {
                            thumb(ui, &t, 56.0);
                            ui.vertical(|ui| {
                                match &mut self.renaming {
                                    Some((old, text)) if old == name => {
                                        let r = ui.add(
                                            egui::TextEdit::singleline(&mut *text)
                                                .desired_width(140.0)
                                                .char_limit(MAX_NAME),
                                        );
                                        if !r.has_focus() && !r.lost_focus() {
                                            r.request_focus(); // once, as it appears
                                        }
                                        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                                            self.renaming = None;
                                        } else if r.lost_focus() {
                                            commit_rename = Some((old.clone(), text.clone()));
                                            self.renaming = None;
                                        }
                                    }
                                    _ => {
                                        let r = ui.add(
                                            egui::Label::new(RichText::new(name).semi())
                                                .sense(Sense::click())
                                                .truncate(),
                                        );
                                        // Nothing else hints that the name can be clicked.
                                        if r.hovered() {
                                            ui.painter().text(
                                                r.rect.right_center() + vec2(6.0, 0.0),
                                                Align2::LEFT_CENTER,
                                                icon::PENCIL_SIMPLE,
                                                FontId::proportional(13.0),
                                                pal().muted,
                                            );
                                        }
                                        if name_it(r, format!("重命名 {name}"))
                                            .on_hover_text(format!("{name} · 点击重命名"))
                                            .clicked()
                                        {
                                            self.renaming = Some((name.clone(), name.clone()));
                                        }
                                    }
                                }
                                let kind = match c.mode {
                                    Mode::Lines => "线条".to_string(),
                                    Mode::Pixels => format!("像素 {0}×{0}", c.grid),
                                    Mode::Image => "图片".to_string(),
                                };
                                // An empty path points at the images folder itself, which exists.
                                if c.mode == Mode::Image
                                    && (c.image.is_empty()
                                        || !crate::config::images_dir().join(&c.image).exists())
                                {
                                    ui.label(
                                        RichText::new("图片文件缺失")
                                            .small()
                                            .color(pal().yellow.1),
                                    );
                                } else {
                                    hint(ui, kind);
                                }
                            });
                        },
                        |ui| {
                            // Right to left, so the first one added sits at the far edge.
                            let armed = self.confirm_delete.as_ref() == Some(name);
                            let text = if armed {
                                format!("{}  确认删除？", icon::TRASH)
                            } else {
                                icon::TRASH.to_string()
                            };
                            // Muted at rest, the danger colour on hover; armed stays danger.
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
                            let r = name_it(
                                btn,
                                if armed {
                                    format!("确认删除 {name}")
                                } else {
                                    format!("删除 {name}")
                                },
                            );
                            let hover = if armed {
                                "再次点击以删除".to_string()
                            } else {
                                format!("删除 {name}")
                            };
                            if r.on_hover_text(hover).clicked() {
                                if armed {
                                    delete = Some(name.clone());
                                } else {
                                    self.confirm_delete = Some(name.clone());
                                }
                            }
                            if icon_button(
                                ui,
                                icon::EXPORT,
                                format!("导出 {name}"),
                                &format!("导出 {name}"),
                            )
                            .clicked()
                            {
                                export = Some(name.clone());
                            }
                            if hk.vk != 0
                                && ghost(ui, icon::X).on_hover_text("清除快捷键").clicked()
                            {
                                hotkey_actions.push((target.clone(), Some(Hotkey::default())));
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
                            } else if hk.vk == 0 {
                                "绑定快捷键".to_string()
                            } else {
                                overlay::key_name(hk)
                            };
                            if name_it(
                                ui.add(egui::Button::new(RichText::new(key_label).small().color(
                                    if listening { pal().accent } else { pal().text },
                                ))),
                                format!("设置预设 {name} 的快捷键"),
                            )
                            .on_hover_text("点击后按下快捷键即可绑定")
                            .clicked()
                            {
                                hotkey_actions.push((target, None));
                            }
                            if *self.local.active_crosshair() == *c {
                                ui.label(
                                    RichText::new(format!("{}  使用中", icon::CHECK))
                                        .color(pal().green.1),
                                );
                            } else if name_it(
                                ui.add(egui::Button::new("加载")),
                                format!("加载 {name}"),
                            )
                            .clicked()
                            {
                                load = Some(name.clone());
                            }
                        },
                    );
                ui.add_space(10.0);
            }
        });
        for (target, set_val) in hotkey_actions {
            match set_val {
                Some(hk) => self.set_hotkey(&target, hk),
                None => self.listening = Some(target),
            }
        }
        if let Some((old, new)) = commit_rename {
            match self.local.rename_preset(&old, &new) {
                Ok(()) => {
                    self.textures.remove(&format!("preset:{old}"));
                    if self.preset_name == old {
                        self.preset_name = new.trim().into();
                    }
                    if self.confirm_delete.as_ref() == Some(&old) {
                        self.confirm_delete = None;
                    }
                }
                Err(e) => self.preset_status = Some(e),
            }
        }
        if let Some(n) = export {
            let one = BTreeMap::from([(n.clone(), self.local.presets[&n].clone())]);
            // Windows won't take these in a file name, and a preset name can hold any of them.
            let file: String = n
                .chars()
                .map(|c| if r#"\/:*?"<>|"#.contains(c) { '_' } else { c })
                .collect();
            self.export_presets(&one, &format!("{file}.opencrosshair"));
        }
        if export_all {
            let all = self.local.presets.clone();
            self.export_presets(&all, "OpenCrosshair 预设合集.opencrosshair");
        }
        if import {
            self.import_presets();
        }
        if let Some(n) = load {
            *self.local.active_crosshair_mut() = self.local.presets[&n].clone();
            self.local.sync_active();
            self.preset_name = n;
        }
        if let Some(n) = delete {
            self.confirm_delete = None;
            self.local.delete_preset(&n);
            self.textures.remove(&format!("preset:{n}"));
        }
    }

    /// Ask where to save, then write the presets to that file.
    fn export_presets(&mut self, presets: &BTreeMap<String, Crosshair>, default_name: &str) {
        let Some(path) = picture::save_share(overlay::settings_window(), default_name) else {
            return;
        };
        self.preset_status = Some(match std::fs::write(&path, share::export(presets)) {
            Ok(()) => "导出成功。".into(),
            Err(_) => "无法保存该文件。".into(),
        });
    }

    /// Ask for a share file and add the presets in it.
    fn import_presets(&mut self) {
        let Some(path) = picture::pick_share(overlay::settings_window()) else {
            return;
        };
        let Ok(text) = std::fs::read_to_string(&path) else {
            self.preset_status = Some("无法读取该文件。".into());
            return;
        };
        self.preset_status = Some(match share::import(&text, &self.local.presets) {
            Ok(new) => {
                let n = new.len();
                self.local.presets.extend(new);
                format!("成功导入 {n} 个预设。")
            }
            Err(e) => e,
        });
    }
}
