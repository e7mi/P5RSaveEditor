use rune::{row, text, El};

use crate::app::{Action, Editor};
use crate::paths::human_size;
use crate::ui::{self, Metrics};

pub fn view(s: &Editor, m: &Metrics) -> El {
    let mut header = super::header_with_actions(
        m,
        "バックアップ",
        "保存前のセーブを自動退避します。復元もここから行えます。",
    );
    header = header
        .child(
            ui::primary(m, "今すぐバックアップ").on_press(|s: &mut Editor| {
                s.request_confirm(
                    Action::CreateBackup,
                    "バックアップ",
                    "現在のセーブのバックアップを作成しますか？",
                )
            }),
        )
        .child(ui::ghost(m, "フォルダを開く").on_press(|s: &mut Editor| s.open_backup_folder()))
        .child(ui::ghost(m, "更新").on_press(|s: &mut Editor| s.refresh_backups()));
    let mut page = super::page_shell(m).child(header);
    if s.backups.is_empty() {
        page = page.child(ui::card(m).child(ui::muted(m, "バックアップはまだありません。")));
    } else {
        let mut list = ui::card(m).gap(m.x(0.0));
        list = list.child(
            row()
                .w_full()
                .padding(m.x(6.0))
                .gap(m.x(8.0))
                .child(
                    text("日時")
                        .font(ui::small_style(m))
                        .color(ui::theme().text_secondary)
                        .w(m.x(138.0)),
                )
                .child(
                    text("スロット")
                        .font(ui::small_style(m))
                        .color(ui::theme().text_secondary)
                        .w(m.x(64.0)),
                )
                .child(
                    text("サイズ")
                        .font(ui::small_style(m))
                        .color(ui::theme().text_secondary)
                        .w(m.x(64.0)),
                )
                .child(
                    text("理由")
                        .font(ui::small_style(m))
                        .color(ui::theme().text_secondary)
                        .grow(1.0),
                )
                .child(text("").font(ui::small_style(m)).w(m.x(48.0))),
        );
        list = list.child(ui::divider());
        for (index, record) in s.backups.iter().enumerate() {
            let record_time = record.display_time();
            let record_slot = record.slot.clone();
            let confirm_time = record_time.clone();
            list = list.child(
                row()
                    .w_full()
                    .min_h(m.x(32.0))
                    .padding(m.x(6.0))
                    .gap(m.x(8.0))
                    .center_y()
                    .radius(m.x(4.0))
                    .hover(ui::theme().hover)
                    .child(
                        text(record_time)
                            .font(m.label())
                            .color(ui::theme().text)
                            .w(m.x(138.0)),
                    )
                    .child(
                        text(record_slot.clone())
                            .font(m.label())
                            .color(ui::accent())
                            .w(m.x(64.0)),
                    )
                    .child(
                        text(human_size(record.size))
                            .font(ui::small_style(m))
                            .color(ui::theme().text_secondary)
                            .w(m.x(64.0)),
                    )
                    .child(
                        text(&record.reason)
                            .font(ui::small_style(m))
                            .color(ui::theme().text_secondary)
                            .grow(1.0)
                            .truncate(),
                    )
                    .child(ui::link(m, "復元").on_press(move |s: &mut Editor| {
                        s.request_confirm(
                            Action::RestoreBackup(index),
                            "バックアップを復元",
                            format!(
                                "{} / {} を復元します。現在のセーブは復元前バックアップとして退避されます。",
                                confirm_time, record_slot
                            ),
                        )
                    })),
            );
        }
        page = page.child(list);
    }
    page.child(ui::small(
        m,
        format!(
            "保存先: {}",
            crate::backups::BackupManager::new(&s.save_dir)
                .root
                .display()
        ),
    ))
}
