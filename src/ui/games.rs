use super::widgets::{
    card, check, empty_state, ghost, heading, hint, name_it, primary, row, segment, texture, thumb,
};
use super::{App, Semi, pal};
use crate::apps;
use crate::config::{Aim, AimButton, AimRule, Game};
use eframe::egui::{self, RichText, TextureHandle, vec2};
use egui_phosphor::bold as icon;
use std::collections::HashMap;

/// A dropdown `width` wide. A long choice ends in "…", and hovering shows it in full.
/// Unlike `set_max_width`, this moves the whole box to a new line when it doesn't fit.
fn dropdown(
    ui: &mut egui::Ui,
    id: impl egui::AsIdSalt,
    width: f32,
    selected: &str,
    add: impl FnOnce(&mut egui::Ui),
) -> egui::Response {
    ui.allocate_ui(vec2(width, ui.spacing().interact_size.y), |ui| {
        egui::ComboBox::from_id_salt(id)
            .selected_text(selected)
            .width(width)
            .truncate()
            .show_ui(ui, add)
            .response
    })
    .inner
    .on_hover_text(selected)
}

/// The three choices that make up an aiming rule. Used for the default and for a game's own.
fn aim_controls(ui: &mut egui::Ui, id: &str, presets: &[String], rule: &mut AimRule) {
    let shown = match &rule.action {
        Aim::Keep => "保持准星不变".to_string(),
        Aim::Hide => "隐藏准星".to_string(),
        Aim::Show => "仅在瞄准时显示".to_string(),
        Aim::Preset(name) => format!("切换到 {name}"),
    };
    egui::Grid::new(("aim_rule", id))
        .num_columns(2)
        .spacing([20.0, 10.0])
        .show(ui, |ui| {
            let label = ui.label(RichText::new("瞄准时动作").color(pal().muted));
            dropdown(ui, (id, "action"), 220.0, &shown, |ui| {
                ui.selectable_value(&mut rule.action, Aim::Keep, "保持准星不变");
                ui.selectable_value(&mut rule.action, Aim::Hide, "隐藏准星");
                ui.selectable_value(&mut rule.action, Aim::Show, "仅在瞄准时显示");
                for name in presets {
                    let choice = format!("切换到 {name}");
                    ui.selectable_value(&mut rule.action, Aim::Preset(name.clone()), choice);
                }
            })
            .labelled_by(label.id);
            ui.end_row();

            // With "Keep the crosshair" nothing changes while aiming, so these two do nothing.
            let live = rule.action != Aim::Keep;
            let label = ui.label(RichText::new("瞄准按键").color(pal().muted));
            ui.add_enabled_ui(live, |ui| {
                egui::ComboBox::from_id_salt((id, "button"))
                    .selected_text(rule.button.label())
                    .width(220.0)
                    .show_ui(ui, |ui| {
                        for b in AimButton::ALL {
                            ui.selectable_value(&mut rule.button, b, b.label());
                        }
                    })
                    .response
                    .labelled_by(label.id);
            });
            ui.end_row();

            row(ui, "", |ui| {
                ui.add_enabled_ui(live, |ui| {
                    check(
                        ui,
                        &mut rule.toggle,
                        "切换模式：按一次开镜，再按一次退出",
                    );
                });
            });
        });
    if rule.action == Aim::Keep {
        ui.add_space(4.0);
        hint(ui, "在上方选择“隐藏准星”、“仅在瞄准时显示”或某个预设后，即可配置瞄准按键与切换模式。");
    }
}

/// What a rule does, in a few words, for showing the default next to a game that follows it.
fn describe_aim(rule: &AimRule) -> String {
    let button = rule.button.label();
    let how = if rule.toggle { "切换" } else { "按住" };
    match &rule.action {
        Aim::Keep => "瞄准时准星保持不变".to_string(),
        Aim::Hide => format!("{how}{button}时隐藏准星"),
        Aim::Show => format!("仅在{how}{button}时显示准星"),
        Aim::Preset(name) => format!("{how}{button}时切换到「{name}」"),
    }
}

/// What the games list shows for an app.
pub(super) struct AppInfo {
    pub(super) name: String,
    icon: Option<TextureHandle>,
    fso: apps::Fso,
}

/// Name and icon for an app, read from its exe once and then cached. Keyed by path, or by the
/// exe name while the path isn't known yet.
fn app_info<'a>(
    cache: &'a mut HashMap<String, AppInfo>,
    ctx: &egui::Context,
    app: &apps::App,
) -> &'a AppInfo {
    let (exe, path) = (app.exe.as_str(), app.path.as_str());
    let key = if path.is_empty() { exe } else { path };
    cache.entry(key.to_string()).or_insert_with(|| AppInfo {
        name: apps::display_name(exe, path, &app.title),
        icon: (!path.is_empty())
            .then(|| apps::icon(path, 48))
            .flatten()
            .map(|(rgba, size)| {
                let image = egui::ColorImage::from_rgba_unmultiplied(size, &rgba);
                ctx.load_texture(format!("icon:{key}"), image, egui::TextureOptions::LINEAR)
            }),
        fso: if path.is_empty() {
            apps::Fso::On
        } else {
            apps::fullscreen_optimizations(path)
        },
    })
}

/// The app's own icon, or a controller when the exe doesn't have one we can read.
fn app_icon(ui: &mut egui::Ui, info: &AppInfo, size: f32) {
    match &info.icon {
        Some(t) => {
            ui.add(egui::Image::from_texture((t.id(), vec2(size, size))));
        }
        None => {
            let glyph = RichText::new(icon::GAME_CONTROLLER)
                .size(size * 0.8)
                .color(pal().muted);
            ui.add_sized([size, size], egui::Label::new(glyph));
        }
    }
}

impl App {
    pub(super) fn games_page(&mut self, ui: &mut egui::Ui) {
        heading(ui, "游戏", "仅在你指定的游戏中自动显示准星。");
        // Games added by typing their exe name get a proper name and icon once they've been seen
        // running.
        for g in &mut self.local.games {
            if let Some(app) = self.running.iter().find(|a| a.exe == g.exe) {
                if g.path.is_empty() {
                    g.path = app.path.clone();
                }
                if g.name.is_empty() {
                    g.name = apps::display_name(&app.exe, &app.path, &app.title);
                }
            }
        }
        let active_fallback = self.local.active_crosshair().clone();
        let s = &mut self.local;

        card(ui, "", |ui| {
            check(
                ui,
                &mut s.only_games,
                RichText::new("仅在我的游戏中显示").semi(),
            );
            let note = if s.only_games {
                "在其他软件中自动隐藏。当前设置窗口位于前台时仍会显示以便预览。"
            } else {
                "当前准星会显示在所有窗口上方。"
            };
            hint(ui, note);
        });

        // The default rule, which every game follows unless it has its own (see its row below).
        let default_rule = AimRule {
            action: s.aim.clone(),
            button: s.aim_button,
            toggle: s.aim_toggle,
        };
        let preset_names: Vec<String> = s.presets.keys().cloned().collect();

        card(ui, "我的游戏", |ui| {
            if s.games.is_empty() {
                empty_state(
                    ui,
                    icon::GAME_CONTROLLER,
                    "暂无游戏",
                    "在下方添加游戏后，准星将仅在你游玩这些游戏时显示。",
                );
            }
            let mut remove = None;
            for (i, g) in s.games.iter_mut().enumerate() {
                let app = apps::App {
                    exe: g.exe.clone(),
                    path: g.path.clone(),
                    title: String::new(),
                };
                let info = app_info(&mut self.apps, ui.ctx(), &app);
                let name = if g.name.is_empty() {
                    &info.name
                } else {
                    &g.name
                };
                let open = self.open_game.as_deref() == Some(g.exe.as_str());
                // Two lines per game: the name with ✕ at the far right, then the settings
                // indented under the name.
                egui::Sides::new().shrink_left().truncate().show(
                    ui,
                    |ui| {
                        app_icon(ui, info, 24.0);
                        let full = if g.path.is_empty() { &g.exe } else { &g.path };
                        ui.add(egui::Label::new(RichText::new(name).semi()).truncate())
                            .on_hover_text(full);
                    },
                    |ui| {
                        if name_it(ghost(ui, icon::X), format!("移除 {name}"))
                            .on_hover_text(format!("移除 {name}"))
                            .clicked()
                        {
                            remove = Some(i);
                        }
                    },
                );
                ui.horizontal(|ui| {
                    // The icon and its spacing, so the settings start under the name. Outside the
                    // wrapping row, so a wrapped line is indented too.
                    ui.add_space(30.0);
                    ui.horizontal_wrapped(|ui| {
                        // On a narrow window whole controls move to a new line; the words in them
                        // don't break ("Windowe / d").
                        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                        // Roomy enough that the checkbox doesn't sit flush against the dropdown.
                        ui.spacing_mut().item_spacing.x = 12.0;
                        ui.spacing_mut().button_padding.x = 6.0;
                        let c = s.presets.get(&g.preset).unwrap_or(&active_fallback);
                        let t =
                            texture(&mut self.textures, ui.ctx(), &format!("game:{}", g.exe), c);
                        thumb(ui, &t, 34.0);
                        // Owned, because the menu below changes g.preset.
                        let shown = if g.preset.is_empty() {
                            "跟随当前准星".to_string()
                        } else {
                            g.preset.clone()
                        };
                        // 140 wide. Wider gaps wrap the whole controls, so the line still breaks cleanly.
                        dropdown(ui, ("preset", i), 140.0, &shown, |ui| {
                            ui.selectable_value(&mut g.preset, String::new(), "跟随当前准星");
                            for name in s.presets.keys() {
                                ui.selectable_value(&mut g.preset, name.clone(), name);
                            }
                        });
                        // "Windowed", not a longer label, so the settings line stays short.
                        let windowed = check(ui, &mut g.windowed, "窗口化").on_hover_text(
                            "仅适用于窗口模式游玩的游戏：以游戏窗口中心为基准，而非显示器屏幕中心。",
                        );
                        // Every row says "Windowed", so say which game each belongs to.
                        let (on, label) = (g.windowed, format!("窗口化：{name}"));
                        windowed.widget_info(|| {
                            egui::WidgetInfo::selected(
                                egui::WidgetType::Checkbox,
                                true,
                                on,
                                label.clone(),
                            )
                        });
                        // This game's own aiming rule: accent when it has one, filled while open.
                        // Plain text colour otherwise, so it reads as a link you can press.
                        let custom = g.aim.is_some();
                        let ink = if custom { pal().accent } else { pal().text };
                        let caret = if open {
                            icon::CARET_UP
                        } else {
                            icon::CARET_DOWN
                        };
                        let aim_text = if custom {
                            "自定义开镜"
                        } else {
                            "默认开镜"
                        };
                        let text = format!("{aim_text} {caret}");
                        let btn = egui::Button::new(RichText::new(text).color(ink))
                            .frame_when_inactive(open);
                        let tip = if custom {
                            "此游戏已设置独立的开镜规则"
                        } else {
                            "此游戏的开镜设置"
                        };
                        // The frame shows on hover (the button's own hover style); pointer says "click".
                        let btn = name_it(ui.add(btn), format!("{name} 的开镜设置"))
                            .on_hover_cursor(egui::CursorIcon::PointingHand);
                        if btn.on_hover_text(tip).clicked() {
                            self.open_game = if open { None } else { Some(g.exe.clone()) };
                        }
                    });
                });
                ui.add_space(8.0); // separates one game from the next
                if open {
                    egui::Frame::new()
                        .fill(pal().field)
                        .corner_radius(8)
                        .inner_margin(egui::Margin::symmetric(16, 14))
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.label(RichText::new(format!("{name} 的开镜设置")).semi());
                            hint(
                                ui,
                                "按下鼠标瞄准键时对该游戏准星的操作。选择“保持准星不变”可在此游戏中忽略瞄准。",
                            );
                            ui.add_space(4.0);
                            ui.horizontal(|ui| {
                                let default = vec2(140.0, 30.0);
                                if segment(ui, "使用默认规则".into(), g.aim.is_none(), default)
                                    .clicked()
                                {
                                    g.aim = None;
                                }
                                let own = vec2(180.0, 30.0);
                                if segment(ui, "为此游戏单独设置".into(), g.aim.is_some(), own)
                                    .clicked()
                                    && g.aim.is_none()
                                {
                                    g.aim = Some(default_rule.clone()); // start from the default
                                }
                            });
                            match &mut g.aim {
                                Some(rule) => aim_controls(ui, &g.exe, &preset_names, rule),
                                None => {
                                    hint(
                                        ui,
                                        format!("当前使用默认规则：{}。", describe_aim(&default_rule)),
                                    );
                                }
                            }
                        });
                    ui.add_space(6.0);
                }
            }
            if let Some(i) = remove {
                if self.open_game.as_deref() == Some(s.games[i].exe.as_str()) {
                    self.open_game = None;
                }
                self.removed_game = Some((i, s.games.remove(i)));
            }
            // Fullscreen optimizations switched off means true exclusive fullscreen, where no
            // overlay can show. Say so, and fix it when it's our account's setting.
            for g in s.games.iter().filter(|g| !g.path.is_empty()) {
                let Some(info) = self.apps.get_mut(&g.path) else {
                    continue;
                };
                if info.fso == apps::Fso::On {
                    continue;
                }
                let name = if g.name.is_empty() {
                    &info.name
                } else {
                    &g.name
                };
                ui.add_space(4.0);
                ui.label(
                    RichText::new(format!(
                        "{}  {name} 已关闭“全屏优化”，在独占全屏模式下无法显示悬浮准星。",
                        icon::WARNING
                    ))
                    .small()
                    .color(pal().yellow.1),
                );
                if info.fso == apps::Fso::OffForEveryone {
                    hint(
                        ui,
                        "该设置对所有账户生效。请在游戏 exe 的 属性 > 兼容性 中取消勾选“禁用全屏优化”（需要管理员权限），或改用无边框窗口模式。",
                    );
                } else if ghost(ui, &format!("{}  重新开启全屏优化", icon::WRENCH)).clicked()
                    && apps::enable_fullscreen_optimizations(&g.path)
                {
                    info.fso = apps::Fso::On;
                }
            }
            // A removal is one click, so offer one line to take it back.
            let mut undo = false;
            if let Some((_, g)) = &self.removed_game {
                let name = if g.name.is_empty() {
                    apps::display_name(&g.exe, "", "")
                } else {
                    g.name.clone()
                };
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    hint(ui, format!("已移除 {name}。"));
                    undo = name_it(ghost(ui, "撤销"), format!("撤销移除 {name}")).clicked();
                });
            }
            if undo && let Some((i, g)) = self.removed_game.take() {
                s.games.insert(i.min(s.games.len()), g);
            }
        });

        // Says whether it added a game, so pressing Enter on an empty box keeps the Undo line.
        let add = |games: &mut Vec<Game>, exe: &str, path: &str, name: &str| -> bool {
            let exe = exe.trim().to_lowercase();
            // "cs2" works as well as "cs2.exe".
            let exe = if exe.is_empty() || exe.ends_with(".exe") {
                exe
            } else {
                format!("{exe}.exe")
            };
            let new = !exe.is_empty() && !games.iter().any(|g| g.exe == exe);
            if new {
                games.push(Game {
                    exe,
                    name: name.to_string(),
                    path: path.to_string(),
                    ..Default::default()
                });
            }
            new
        };
        card(ui, "添加游戏", |ui| {
            ui.horizontal(|ui| {
                let mut picked = None;
                egui::ComboBox::from_id_salt("running")
                    .selected_text("选择正在运行的程序…")
                    .width(260.0)
                    .show_ui(ui, |ui| {
                        for (i, app) in self.running.iter().enumerate() {
                            let info = app_info(&mut self.apps, ui.ctx(), app);
                            let btn = match &info.icon {
                                Some(t) => egui::Button::image_and_text(
                                    egui::Image::from_texture((t.id(), vec2(18.0, 18.0))),
                                    info.name.as_str(),
                                ),
                                None => egui::Button::new(info.name.as_str()),
                            };
                            let btn = btn.frame_when_inactive(false).min_size(vec2(240.0, 0.0));
                            if ui.add(btn).on_hover_text(&app.exe).clicked() {
                                picked = Some(i);
                            }
                        }
                    });
                if let Some(app) = picked.map(|i| &self.running[i]) {
                    let name = app_info(&mut self.apps, ui.ctx(), app).name.clone();
                    if add(&mut s.games, &app.exe, &app.path, &name) {
                        self.removed_game = None;
                    }
                }
                if ghost(ui, &format!("{}  刷新", icon::ARROWS_CLOCKWISE)).clicked() {
                    self.running = apps::running();
                }
            });
            ui.horizontal(|ui| {
                let r = ui.add(
                    egui::TextEdit::singleline(&mut self.game_input)
                        .hint_text("或手动输入 exe 名称，例如 cs2")
                        .desired_width(260.0),
                );
                let enter = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                if primary(
                    ui,
                    !self.game_input.trim().is_empty(),
                    &format!("{}  添加", icon::PLUS),
                )
                .clicked()
                    || enter
                {
                    if add(&mut s.games, &self.game_input, "", "") {
                        self.removed_game = None;
                    }
                    self.game_input.clear();
                }
            });
            hint(ui, "请先打开游戏，再点击“刷新”从列表中选择。");
        });

        card(ui, "瞄准时行为", |ui| {
            let mut rule = default_rule.clone();
            aim_controls(ui, "default", &preset_names, &mut rule);
            (s.aim, s.aim_button, s.aim_toggle) = (rule.action, rule.button, rule.toggle);
            hint(ui, "适用于开镜瞄准类游戏：仅轮询按键状态，不会向游戏注入任何输入钩子。");
            hint(ui, "此项为所有游戏的默认规则；如需为单个游戏单独配置，请点击「我的游戏」对应行中的“默认开镜”。");
        });
    }
}
