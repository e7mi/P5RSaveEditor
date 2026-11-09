#![cfg_attr(windows, windows_subsystem = "windows")]

//! P5R save editor, native Rust UI built with Rune.

use p5r_save_editor::app;
use rune::AppSpec;

#[cfg(windows)]
fn set_window_icon(window: &rune::WindowHandle) {
    use rune::raw_window_handle::RawWindowHandle;
    use winapi::um::libloaderapi::GetModuleHandleW;
    use winapi::um::winuser::{LoadIconW, SendMessageW, ICON_BIG, ICON_SMALL, WM_SETICON};

    let _ = window.with_raw_handle(|raw| {
        if let RawWindowHandle::Win32(win32) = raw.as_raw() {
            let hwnd = win32.hwnd.get() as winapi::shared::windef::HWND;
            unsafe {
                let module = GetModuleHandleW(std::ptr::null());
                let icon = LoadIconW(module, std::ptr::without_provenance::<u16>(1));
                if icon.is_null() {
                    return;
                }
                SendMessageW(hwnd, WM_SETICON, ICON_SMALL as usize, icon as isize);
                SendMessageW(hwnd, WM_SETICON, ICON_BIG as usize, icon as isize);
            }
        }
    });
}

fn main() {
    let spec = AppSpec::<app::Editor> {
        size: (1280.0, 820.0),
        min_size: Some((1080.0, 680.0)),
        fonts: &[include_bytes!("../assets/MPLUS1p-Regular.ttf")],
        families: ("M PLUS 1p", "M PLUS 1p"),
        scenes: &[
            ("basic", |s: &mut app::Editor| s.set_page(app::Page::Basic)),
            ("confidants", |s: &mut app::Editor| {
                s.set_page(app::Page::Confidants)
            }),
            ("members", |s: &mut app::Editor| {
                s.set_page(app::Page::Members)
            }),
            ("inventory", |s: &mut app::Editor| {
                s.set_page(app::Page::Inventory)
            }),
            ("compendium", |s: &mut app::Editor| {
                s.set_page(app::Page::Compendium)
            }),
            ("backups", |s: &mut app::Editor| {
                s.set_page(app::Page::Backups)
            }),
            ("settings", |s: &mut app::Editor| {
                s.set_page(app::Page::Settings)
            }),
            ("advanced", |s: &mut app::Editor| {
                s.set_page(app::Page::Basic);
                if !s.advanced() {
                    s.toggle_advanced();
                }
            }),
            ("persona-modal", |s: &mut app::Editor| {
                s.set_page(app::Page::Members);
                s.open_persona(0, 1);
            }),
            ("slots-modal", |s: &mut app::Editor| {
                s.modal = Some(app::Modal::Slots)
            }),
            ("confirm", |s: &mut app::Editor| {
                s.request_confirm(app::Action::MaxMoney, "確認", "所持金を MAX にしますか？")
            }),
            ("picker", |s: &mut app::Editor| {
                s.set_page(app::Page::Members);
                s.open_persona(0, 1);
                s.open_picker(app::PickerKind::Skill, app::PickerTarget::Skill(0));
            }),
        ],
        ..AppSpec::new(
            "P5R Save Editor",
            (1280.0, 820.0),
            app::Editor::new,
            app::view,
        )
    };
    let app = rune::App::new(spec);
    #[cfg(windows)]
    let app = app.on_window(set_window_icon);
    app.run();
}
