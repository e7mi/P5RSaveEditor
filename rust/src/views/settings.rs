use rune::{row, text, Cursor, El};

use crate::app::{Action, Editor};
use crate::paths::{display_path, APP_VERSION};
use crate::ui::{self, Metrics};

pub fn view(s: &Editor, m: &Metrics) -> El {
    let t = ui::theme();
    let mut page = super::page_shell(m).child(super::header_with_actions(
        m,
        "設定",
        "セーブフォルダ・バックアップ・表示の設定を変更します。",
    ));

    let mut folder = ui::card(m);
    folder = folder
        .child(super::card_header(m, "セーブフォルダ"))
        .child(
            text(display_path(&s.save_dir))
                .font(m.body())
                .color(t.text)
                .truncate()
                .help_when_truncated(display_path(&s.save_dir)),
        )
        .child(
            row()
                .w_full()
                .gap(m.x(8.0))
                .child(
                    ui::primary(m, "フォルダを変更").on_press(|s: &mut Editor| s.choose_folder()),
                )
                .child(ui::ghost(m, "自動検出").on_press(|s: &mut Editor| {
                    s.request_confirm(
                        Action::AutoDetect,
                        "自動検出",
                        "P5R のセーブフォルダを自動検出して切り替えますか？",
                    )
                }))
                .child(
                    ui::ghost(m, "エクスプローラーで開く")
                        .on_press(|s: &mut Editor| crate::paths::open_in_explorer(&s.save_dir)),
                ),
        );
    page = page.child(folder);

    let mut options = ui::card(m);
    options = options
        .child(super::card_header(m, "動作"))
        .child(setting_toggle(
            m,
            s.settings.auto_backup,
            "保存前に自動バックアップ",
            "保存直前に元の DATA.DAT を退避します。",
            |s: &mut Editor| s.toggle_auto_backup(),
        ))
        .child(setting_toggle(
            m,
            s.settings.show_reserve,
            "予備スロットを表示",
            "未使用・予備のアイテム行も一覧に表示します。",
            |s: &mut Editor| s.toggle_show_reserve(),
        ))
        .child(setting_toggle(
            m,
            s.settings.advanced,
            "高度なモード",
            "日付変更や一括操作など、危険な操作を表示します。",
            |s: &mut Editor| s.toggle_advanced(),
        ));
    if s.advanced() {
        options = options.child(setting_toggle(
            m,
            s.settings.allow_save_while_running,
            "ゲーム起動中も保存を許可",
            "非推奨: Steam Cloud の競合やセーブ破損の原因になります。",
            |s: &mut Editor| s.toggle_allow_running(),
        ));
    }
    page = page.child(options);

    let mut data = ui::card(m);
    data = data
        .child(super::card_header(m, "データ"))
        .child(
            row()
                .w_full()
                .gap(m.x(8.0))
                .child(
                    ui::ghost(m, "バックアップフォルダ")
                        .on_press(|s: &mut Editor| s.open_backup_folder()),
                )
                .child(ui::ghost(m, "アプリデータ").on_press(|s: &mut Editor| s.open_data_folder())),
        )
        .child(ui::small(
            m,
            "対応形式: Steam PC ネイティブセーブ (DATA.DAT, payload 0x31)。PS4 変換セーブ (0x2D) は非対応です。",
        ));
    page = page.child(data);

    let mut about = ui::card(m);
    about = about
        .child(super::card_header(m, "このアプリについて"))
        .child(ui::body(m, format!("P5R Save Editor v{APP_VERSION}")))
        .child(ui::small(
            m,
            "ペルソナ・スキル・装備・アイテム名は日本語表記です。",
        ))
        .child(ui::small(
            m,
            "実機でのロード確認は行っていません。初回は必ずバックアップを取り、ゲーム内でロードを確認してください。",
        ))
        .child(ui::small(
            m,
            "非公式ツールです。ATLUS / SEGA とは無関係です。Persona 5 Royal は ATLUS / SEGA の商標です。",
        ));
    page.child(about)
}

fn setting_toggle(
    m: &Metrics,
    checked: bool,
    label: &str,
    detail: &str,
    on_press: impl Fn(&mut Editor) + 'static,
) -> El {
    let t = ui::theme();
    row()
        .w_full()
        .min_h(m.x(38.0))
        .padding(m.x(6.0))
        .gap(m.x(8.0))
        .center_y()
        .radius(m.x(5.0))
        .hover(t.hover)
        .cursor(Cursor::Pointer)
        .on_press(move |s: &mut Editor| on_press(s))
        .child(ui::checkbox(m, checked))
        .child(
            rune::col()
                .gap(m.x(1.0))
                .child(ui::body(m, label))
                .child(ui::small(m, detail)),
        )
}
