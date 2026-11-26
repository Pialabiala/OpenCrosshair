use super::widgets::{card, check, ghost, heading, hint, name_it, segment};
use super::{App, ListenTarget, Semi, pal};
use crate::config::{Hotkey, Theme};
use crate::overlay;
use crate::{install, update};
use eframe::egui::{self, Align2, FontId, RichText, Sense, Stroke, StrokeKind, vec2};
use egui_phosphor::bold as icon;

/// Keyboard key rendered as a keycap.
fn keycap(ui: &mut egui::Ui, text: &str, active: bool) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(190.0, 34.0), Sense::click());
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, text));
    let p = pal();
    let (fill, stroke) = if active {
        (p.accent.gamma_multiply(0.18), p.accent)
    } else if resp.is_pointer_button_down_on() {
        (p.press, p.border)
    } else if resp.hovered() {
        (p.hover, p.border)
    } else {
        (p.field, p.border)
    };
    let painter = ui.painter();
    painter.rect(rect, 6, fill, Stroke::new(1.0, stroke), StrokeKind::Inside);
    if resp.has_focus() {
        painter.rect_stroke(rect, 6, Stroke::new(1.5, p.accent), StrokeKind::Inside);
    }
    // Monospace fonts reserve more room below the baseline, which makes centred text look high.
    let nudge = vec2(0.0, 2.5);
    painter.text(
        rect.center() + nudge,
        Align2::CENTER_CENTER,
        text,
        FontId::monospace(13.5),
        p.text,
    );
    resp
}

impl App {
    fn hotkey_row(&mut self, ui: &mut egui::Ui, label: &str, target: ListenTarget) {
        let action = ui.label(label);
        let hk = self.get_hotkey(&target);
        let listening = self.listening.as_ref() == Some(&target);
        let text = if listening {
            "请按下按键…".into()
        } else {
            overlay::key_name(hk)
        };
        if keycap(ui, &text, listening).labelled_by(action.id).clicked() {
            self.listening = Some(target.clone());
        }
        if name_it(ghost(ui, icon::X), format!("解绑 {label}"))
            .on_hover_text(format!("清除 {label} 快捷键"))
            .clicked()
        {
            self.set_hotkey(&target, Hotkey::default());
        }
        if let Some(why) = self.hotkey_issue(&target, hk) {
            ui.label(
                RichText::new(format!("{}  {why}", icon::WARNING))
                    .small()
                    .color(pal().yellow.1),
            );
        }
        ui.end_row();
    }

    pub(super) fn settings_page(&mut self, ui: &mut egui::Ui) {
        heading(ui, "设置", "外观、快捷键、开机启动与自动更新。");
        card(ui, "外观", |ui| {
            // Wraps onto a second line when the text size leaves no room for three across.
            ui.horizontal_wrapped(|ui| {
                for (t, ic, name) in [
                    (Theme::System, icon::MONITOR, "跟随系统"),
                    (Theme::Light, icon::SUN, "浅色"),
                    (Theme::Dark, icon::MOON, "深色"),
                ] {
                    let on = self.local.theme == t;
                    if segment(ui, format!("{ic}  {name}"), on, vec2(110.0, 34.0)).clicked() {
                        self.local.theme = t;
                    }
                }
            });
            hint(ui, "跟随系统将自动匹配 Windows 的浅色或深色应用模式。");
        });
        card(ui, "快捷键", |ui| {
            hint(ui, "全局快捷键");
            ui.add_space(4.0);
            egui::Grid::new("global_keys")
                .num_columns(4)
                .spacing([20.0, 12.0])
                .show(ui, |ui| {
                    for (label, target) in [
                        ("显示 / 隐藏准星", ListenTarget::Toggle),
                        ("显示 / 隐藏设置窗口", ListenTarget::Menu),
                        ("切换到上一个准星", ListenTarget::PrevCrosshair),
                        ("切换到下一个准星", ListenTarget::NextCrosshair),
                    ] {
                        self.hotkey_row(ui, label, target);
                    }
                });

            ui.add_space(12.0);
            hint(ui, "独立准星快捷键（按下可切换并显示对应准星，再次按下隐藏）");
            ui.add_space(4.0);
            let crosshair_names: Vec<String> = self
                .local
                .crosshairs
                .iter()
                .map(|c| c.name.clone())
                .collect();
            egui::Grid::new("crosshair_keys")
                .num_columns(4)
                .spacing([20.0, 12.0])
                .show(ui, |ui| {
                    for (i, name) in crosshair_names.into_iter().enumerate() {
                        self.hotkey_row(ui, &name, ListenTarget::Crosshair(i));
                    }
                });

            if !self.local.presets.is_empty() {
                ui.add_space(12.0);
                hint(ui, "预设快捷键（按下可一键将当前准星切换为对应预设）");
                ui.add_space(4.0);
                let preset_names: Vec<String> = self.local.presets.keys().cloned().collect();
                egui::Grid::new("preset_keys")
                    .num_columns(4)
                    .spacing([20.0, 12.0])
                    .show(ui, |ui| {
                        for name in preset_names {
                            self.hotkey_row(ui, &name, ListenTarget::Preset(name.clone()));
                        }
                    });
            }

            ui.add_space(6.0);
            hint(ui, format!("{}  点击按键框后按下任意按键或组合键即可绑定，按 Esc 取消。", icon::KEYBOARD));
        });
        card(ui, "开机启动", |ui| {
            ui.horizontal(|ui| {
                if check(
                    ui,
                    &mut self.autostart,
                    RichText::new("随 Windows 开机自启").semi(),
                )
                .changed()
                {
                    install::set_autostart(self.autostart);
                    self.autostart = install::autostart(); // show what actually got written
                }
            });
            // An unbound key would read "Press Not set", so leave it out.
            let how = if self.local.menu_key.vk == 0 {
                "点击系统托盘图标可打开设置。".to_string()
            } else {
                format!(
                    "按 {} 或点击系统托盘图标可打开设置。",
                    overlay::key_name(self.local.menu_key)
                )
            };
            hint(ui, format!("{}  开机后在系统托盘静默运行。{how}", icon::ROCKET_LAUNCH));
        });
        card(ui, "更新", |ui| {
            ui.horizontal(|ui| {
                check(
                    ui,
                    &mut self.local.auto_update,
                    RichText::new("自动安装更新").semi(),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        RichText::new(format!("v{}", update::VERSION))
                            .monospace()
                            .color(pal().muted),
                    );
                });
            });
            let (text, busy) = match update::STATUS.lock().unwrap().clone() {
                update::Status::Idle => ("尚未检查更新。".to_string(), false),
                update::Status::DevBuild => ("开发版本中已禁用自动更新。".to_string(), true),
                update::Status::Checking => ("正在检查 GitHub 更新…".to_string(), true),
                update::Status::UpToDate => ("当前已是最新版本。".to_string(), false),
                update::Status::Downloading(v) => (format!("正在下载 {v}…"), true),
                update::Status::WaitingForGame(v) => (
                    format!("{v} 已下载就绪，将在你退出游戏后自动安装。"),
                    true,
                ),
                update::Status::Failed(e) => (format!("上次检查失败：{e}"), false),
            };
            ui.horizontal(|ui| {
                let check = egui::Button::new(format!("{}  立即检查", icon::ARROWS_CLOCKWISE));
                if ui.add_enabled(!busy, check).clicked() {
                    let _ = self.updates.send(());
                }
                hint(ui, text);
            });
            hint(ui, "更新来自项目的 GitHub Releases，游玩期间绝不会强制重启。");
        });
        card(ui, "关于", |ui| {
            let repo = env!("CARGO_PKG_REPOSITORY");
            hint(ui, "基于 MIT 开源协议免费发布。");
            ui.horizontal(|ui| {
                if ghost(ui, &format!("{}  GitHub 源码", icon::GITHUB_LOGO)).clicked() {
                    install::open_url(repo);
                }
                if ghost(ui, &format!("{}  反馈问题", icon::BUG)).clicked() {
                    install::open_url(&format!("{repo}/issues/new"));
                }
            });
        });
    }
}
