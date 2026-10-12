//! CraftSuite: ein Startfenster für FilmCraft und EffectCraft.
//!
//! Beim Start zeigt CraftSuite beide Apps zur Auswahl; ein Klick startet die gewählte App und
//! schließt das Fenster. Die Apps selbst sind die unveränderten Originale, die neben
//! `CraftSuite.exe` in `FilmCraft/` und `EffectCraft/` liegen, daher haben sie alle ihre
//! Funktionen eins zu eins.
//!
//! Ohne Fenster geht es direkt, wenn klar ist, welche App gemeint ist:
//! `CraftSuite --filmcraft …`, `CraftSuite --effectcraft …`, oder eine Projektdatei
//! (`.fcproj` → FilmCraft, `.ecproj` / `.ecprojx` → EffectCraft). Alle übrigen Argumente gehen
//! unverändert an die gestartete App.

#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use egui::{Align2, Color32, CornerRadius, FontId, Key, Rect, Sense, Stroke, StrokeKind, TextureHandle, vec2};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

const BG: Color32 = Color32::from_rgb(0x15, 0x13, 0x1b);
const CARD: Color32 = Color32::from_rgb(0x21, 0x1e, 0x2a);
const CARD_HOVER: Color32 = Color32::from_rgb(0x2a, 0x26, 0x36);
const TEXT: Color32 = Color32::from_rgb(0xee, 0xea, 0xf6);
const MUTED: Color32 = Color32::from_rgb(0x9a, 0x94, 0xab);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Choice {
    Film,
    Effect,
}

struct AppInfo {
    name: &'static str,
    /// Folder next to the launcher and the executable's file stem inside it.
    folder: &'static str,
    exe: &'static str,
    /// Environment variable that overrides the executable's path (development, custom installs).
    env: &'static str,
    version: &'static str,
    tagline: &'static str,
    features: &'static [&'static str],
    accent: Color32,
    icon_png: &'static [u8],
}

const FILM: AppInfo = AppInfo {
    name: "FilmCraft",
    folder: "FilmCraft",
    exe: "filmcraft",
    env: "CRAFTSUITE_FILMCRAFT",
    version: env!("FILMCRAFT_VERSION"),
    tagline: "Videoschnitt, Farbe und Ton",
    features: &["Schnitt wie in Premiere Pro", "Lumetri-Farbkorrektur", "Audio-Mixer und Untertitel"],
    accent: Color32::from_rgb(0x8b, 0x5c, 0xf6),
    icon_png: include_bytes!("../assets/filmcraft.png"),
};

const EFFECT: AppInfo = AppInfo {
    name: "EffectCraft",
    folder: "EffectCraft",
    exe: "effectcraft",
    env: "CRAFTSUITE_EFFECTCRAFT",
    version: env!("EFFECTCRAFT_VERSION"),
    tagline: "Motion Graphics und visuelle Effekte",
    features: &["Kompositionen wie in After Effects", "306 Effekte und Expressions", "3D-Kameras und Lichter"],
    accent: Color32::from_rgb(0xe0, 0x3c, 0x9a),
    icon_png: include_bytes!("../assets/effectcraft.png"),
};

impl Choice {
    const ALL: [Choice; 2] = [Choice::Film, Choice::Effect];

    fn info(self) -> &'static AppInfo {
        match self {
            Choice::Film => &FILM,
            Choice::Effect => &EFFECT,
        }
    }

    /// The app that owns a project file, by its extension.
    fn for_file(path: &Path) -> Option<Choice> {
        let ext = path.extension()?.to_str()?;
        if ext.eq_ignore_ascii_case("fcproj") {
            Some(Choice::Film)
        } else if ext.eq_ignore_ascii_case("ecproj") || ext.eq_ignore_ascii_case("ecprojx") {
            Some(Choice::Effect)
        } else {
            None
        }
    }

    fn other(self) -> Choice {
        match self {
            Choice::Film => Choice::Effect,
            Choice::Effect => Choice::Film,
        }
    }
}

/// Where the app's executable is: `$CRAFTSUITE_FILMCRAFT` / `$CRAFTSUITE_EFFECTCRAFT` if set,
/// otherwise `<folder of CraftSuite>/<App>/<app>.exe`.
fn exe_path(choice: Choice) -> PathBuf {
    let info = choice.info();
    if let Some(p) = std::env::var_os(info.env).filter(|p| !p.is_empty()) {
        return PathBuf::from(p);
    }
    let dir = std::env::current_exe().ok().and_then(|p| p.parent().map(Path::to_path_buf)).unwrap_or_default();
    dir.join(info.folder).join(format!("{}{}", info.exe, std::env::consts::EXE_SUFFIX))
}

/// Starts the app as its own process, passing `args` through, and returns once it is running.
fn launch(choice: Choice, args: &[OsString]) -> Result<(), String> {
    let info = choice.info();
    let exe = exe_path(choice);
    if !exe.is_file() {
        return Err(format!("{} wurde nicht gefunden:\n{}", info.name, exe.display()));
    }
    let mut cmd = Command::new(&exe);
    cmd.args(args);
    if let Some(dir) = exe.parent() {
        cmd.current_dir(dir);
    }
    cmd.spawn().map(drop).map_err(|e| format!("{} konnte nicht gestartet werden:\n{e}", info.name))
}

/// Command line: an explicit `--filmcraft` / `--effectcraft`, or the app a project file belongs to.
struct Launch {
    choice: Option<Choice>,
    /// Everything else, for the app (relative paths made absolute: the app starts in its folder).
    args: Vec<OsString>,
}

fn parse_args(raw: impl IntoIterator<Item = OsString>) -> Result<Launch, String> {
    let cwd = std::env::current_dir().unwrap_or_default();
    let mut choice = None;
    let mut by_file = None;
    let mut args = Vec::new();
    for arg in raw {
        match arg.to_str() {
            Some("--filmcraft") => choice = Some(Choice::Film),
            Some("--effectcraft") => choice = Some(Choice::Effect),
            Some("-h" | "--help") => return Err(HELP.into()),
            Some("-V" | "--version") => {
                return Err(format!("CraftSuite {}\nFilmCraft {}\nEffectCraft {}", env!("CARGO_PKG_VERSION"), FILM.version, EFFECT.version));
            }
            Some(s) if s.starts_with('-') => args.push(arg),
            _ => {
                let path = Path::new(&arg);
                by_file = by_file.or_else(|| Choice::for_file(path));
                args.push(if path.is_relative() && cwd.join(path).exists() { cwd.join(path).into_os_string() } else { arg });
            }
        }
    }
    Ok(Launch { choice: choice.or(by_file), args })
}

const HELP: &str = "CraftSuite: FilmCraft und EffectCraft in einer App.

Aufruf: CraftSuite [--filmcraft | --effectcraft] [Argumente der App …]

Ohne Angabe erscheint das Startfenster. Eine Projektdatei wählt die App von selbst:
  .fcproj            FilmCraft
  .ecproj, .ecprojx  EffectCraft
Alle anderen Argumente gehen unverändert an die gestartete App.";

struct Picker {
    icons: [TextureHandle; 2],
    selected: Choice,
    args: Vec<OsString>,
    error: Option<String>,
}

impl Picker {
    fn new(cc: &eframe::CreationContext<'_>, args: Vec<OsString>, error: Option<String>) -> Self {
        let ctx = &cc.egui_ctx;
        ctx.set_visuals(egui::Visuals::dark());
        let icon = |info: &AppInfo| {
            let image = image::load_from_memory(info.icon_png).map(|i| i.to_rgba8()).unwrap_or_default();
            let size = [image.width() as usize, image.height() as usize];
            ctx.load_texture(info.name, egui::ColorImage::from_rgba_unmultiplied(size, image.as_raw()), egui::TextureOptions::LINEAR)
        };
        Picker { icons: [icon(&FILM), icon(&EFFECT)], selected: Choice::Film, args, error }
    }

    fn start(&mut self, ctx: &egui::Context, choice: Choice) {
        match launch(choice, &self.args) {
            Ok(()) => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
            Err(e) => self.error = Some(e),
        }
    }

    /// One app's card; returns true when it was clicked.
    fn card(&mut self, ui: &mut egui::Ui, choice: Choice, size: egui::Vec2) -> bool {
        let info = choice.info();
        let (rect, response) = ui.allocate_exact_size(size, Sense::click());
        let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
        if response.hovered() {
            self.selected = choice;
        }
        let active = self.selected == choice;
        let t = ui.ctx().animate_bool(response.id, active);
        let rect = rect.translate(vec2(0.0, -4.0 * t));
        let painter = ui.painter_at(rect.expand(8.0));

        let accent = info.accent;
        painter.rect(
            rect,
            CornerRadius::same(18),
            if active { CARD_HOVER } else { CARD },
            Stroke::new(1.0 + 1.5 * t, if active { accent } else { Color32::from_white_alpha(18) }),
            StrokeKind::Inside,
        );

        let icon_size = 128.0;
        let icon_rect = Rect::from_center_size(rect.center_top() + vec2(0.0, 28.0 + icon_size / 2.0), vec2(icon_size, icon_size));
        let index = Choice::ALL.iter().position(|c| *c == choice).unwrap_or(0);
        painter.image(self.icons[index].id(), icon_rect, Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), Color32::WHITE);

        let mut y = icon_rect.bottom() + 22.0;
        painter.text(egui::pos2(rect.center().x, y), Align2::CENTER_TOP, info.name, FontId::proportional(26.0), TEXT);
        y += 34.0;
        painter.text(egui::pos2(rect.center().x, y), Align2::CENTER_TOP, info.tagline, FontId::proportional(15.0), accent.gamma_multiply(0.7 + 0.3 * t));
        y += 30.0;
        for feature in info.features {
            painter.text(egui::pos2(rect.center().x, y), Align2::CENTER_TOP, *feature, FontId::proportional(13.5), MUTED);
            y += 20.0;
        }

        // Key hint in the corner, version in the other.
        let key = format!("{}", index + 1);
        let badge = Rect::from_min_size(rect.left_top() + vec2(14.0, 14.0), vec2(24.0, 24.0));
        painter.rect(badge, CornerRadius::same(6), Color32::from_white_alpha(10), Stroke::new(1.0, Color32::from_white_alpha(30)), StrokeKind::Inside);
        painter.text(badge.center(), Align2::CENTER_CENTER, key, FontId::monospace(13.0), MUTED);
        painter.text(rect.right_top() + vec2(-16.0, 18.0), Align2::RIGHT_TOP, format!("v{}", info.version), FontId::proportional(12.0), MUTED);

        response.clicked()
    }
}

impl eframe::App for Picker {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let (left, right, enter, escape, one, two) = ctx.input(|i| {
            (
                i.key_pressed(Key::ArrowLeft),
                i.key_pressed(Key::ArrowRight) || i.key_pressed(Key::Tab),
                i.key_pressed(Key::Enter) || i.key_pressed(Key::Space),
                i.key_pressed(Key::Escape),
                i.key_pressed(Key::Num1),
                i.key_pressed(Key::Num2),
            )
        });
        if escape {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        if left || right {
            self.selected = self.selected.other();
        }
        let mut chosen = if one {
            Some(Choice::Film)
        } else if two {
            Some(Choice::Effect)
        } else if enter {
            Some(self.selected)
        } else {
            None
        };

        egui::CentralPanel::default().frame(egui::Frame::NONE.fill(BG)).show(ui, |ui| {
            let full = ui.max_rect();
            ui.painter().text(full.center_top() + vec2(0.0, 30.0), Align2::CENTER_TOP, "CraftSuite", FontId::proportional(30.0), TEXT);
            ui.painter().text(full.center_top() + vec2(0.0, 70.0), Align2::CENTER_TOP, "Welche App möchtest du starten?", FontId::proportional(15.0), MUTED);

            let card = vec2(330.0, 340.0);
            let gap = 28.0;
            let row = Rect::from_center_size(egui::pos2(full.center().x, full.top() + 112.0 + card.y / 2.0), vec2(card.x * 2.0 + gap, card.y));
            let mut row_ui = ui.new_child(egui::UiBuilder::new().max_rect(row).layout(egui::Layout::left_to_right(egui::Align::Center)));
            row_ui.spacing_mut().item_spacing.x = gap;
            for choice in Choice::ALL {
                if self.card(&mut row_ui, choice, card) {
                    chosen = Some(choice);
                }
            }

            let footer = egui::pos2(full.center().x, full.bottom() - 26.0);
            if let Some(error) = &self.error {
                ui.painter().text(footer - vec2(0.0, 10.0), Align2::CENTER_BOTTOM, error, FontId::proportional(13.0), Color32::from_rgb(0xff, 0x6b, 0x6b));
            } else {
                let hint = if self.args.is_empty() { "Klicken, 1 / 2 drücken oder Pfeiltasten und Enter · Esc beendet" } else { "Die übergebenen Dateien werden in der gewählten App geöffnet · Esc beendet" };
                ui.painter().text(footer, Align2::CENTER_CENTER, hint, FontId::proportional(12.5), MUTED);
            }
        });

        if let Some(choice) = chosen {
            self.selected = choice;
            self.start(&ctx, choice);
        }
    }
}

fn main() -> eframe::Result {
    let launch_args = match parse_args(std::env::args_os().skip(1)) {
        Ok(l) => l,
        Err(message) => {
            println!("{message}");
            return Ok(());
        }
    };
    // The app is known from the command line: start it without showing the picker. If that
    // fails, the picker opens with the error so the user can see what went wrong.
    let mut error = None;
    if let Some(choice) = launch_args.choice {
        match launch(choice, &launch_args.args) {
            Ok(()) => return Ok(()),
            Err(e) => error = Some(e),
        }
    }

    let icon = eframe::icon_data::from_png_bytes(include_bytes!("../assets/craftsuite-256.png")).unwrap_or_default();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("CraftSuite")
            .with_inner_size([760.0, 520.0])
            .with_resizable(false)
            .with_maximize_button(false)
            .with_icon(icon)
            .with_app_id("ai.storyteller.craftsuite"),
        centered: true,
        ..Default::default()
    };
    let args = launch_args.args;
    eframe::run_native("CraftSuite", options, Box::new(move |cc| Ok(Box::new(Picker::new(cc, args, error)))))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Launch {
        parse_args(args.iter().map(OsString::from)).unwrap_or_else(|e| panic!("{e}"))
    }

    #[test]
    fn project_files_pick_their_app() {
        assert_eq!(parse(&["cut.fcproj"]).choice, Some(Choice::Film));
        assert_eq!(parse(&["Intro.ECPROJ"]).choice, Some(Choice::Effect));
        assert_eq!(parse(&["intro.ecprojx"]).choice, Some(Choice::Effect));
        assert_eq!(parse(&["clip.mp4"]).choice, None);
        assert_eq!(parse(&[]).choice, None);
    }

    #[test]
    fn explicit_flag_wins_and_is_not_passed_on() {
        let l = parse(&["--effectcraft", "cut.fcproj", "--demo"]);
        assert_eq!(l.choice, Some(Choice::Effect));
        assert_eq!(l.args, [OsString::from("cut.fcproj"), OsString::from("--demo")]);
    }

    #[test]
    fn help_and_version_do_not_launch() {
        assert!(parse_args([OsString::from("--help")]).is_err());
        assert!(parse_args([OsString::from("-V")]).is_err());
    }

    #[test]
    fn exe_lives_in_the_app_folder() {
        let p = exe_path(Choice::Film);
        assert!(p.ends_with(Path::new("FilmCraft").join(format!("filmcraft{}", std::env::consts::EXE_SUFFIX))) || std::env::var_os(FILM.env).is_some());
    }
}
