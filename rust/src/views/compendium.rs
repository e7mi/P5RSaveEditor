use rune::{row, spacer, text, Cursor, El};

use crate::app::{Action, Editor};
use crate::tables::tables;
use crate::ui::{self, Metrics};

pub fn view(s: &Editor, m: &Metrics) -> El {
    let Some(doc) = &s.doc else {
        return ui::empty_state(m, "DATA01〜DATA16 のいずれかを読み込んでください。");
    };
    let mut header =
        super::header_with_actions(m, "図鑑", "ペルソナ図鑑の登録ビットを編集します。");
    if s.advanced() {
        header = header
            .child(ui::danger_button(m, "全登録").on_press(|s: &mut Editor| {
                s.request_confirm(
                    Action::AllCompendium,
                    "図鑑 全登録",
                    "232 件すべての登録ビットを有効にします。実行しますか？",
                )
            }))
            .child(ui::danger_button(m, "全解除").on_press(|s: &mut Editor| {
                s.request_confirm(
                    Action::ClearCompendium,
                    "図鑑 全解除",
                    "232 件すべての登録ビットを解除します。実行しますか？",
                )
            }));
    }

    let mut controls = row().w_full().gap(m.x(8.0)).center_y();
    controls = controls.child(
        row()
            .h(m.x(26.0))
            .w(m.x(220.0))
            .px(m.x(7.0))
            .gap(m.x(6.0))
            .center_y()
            .radius(m.x(5.0))
            .fill(ui::theme().control_track)
            .border(1.0, ui::theme().card_border)
            .child(ui::small(m, "検索"))
            .child(
                rune::text_input(&s.compendium_search, m.body())
                    .single_line()
                    .grow(1.0)
                    .h(m.x(24.0))
                    .on_edit(|s: &mut Editor| &mut s.compendium_search)
                    .on_input_change(|s: &mut Editor, _| s.compendium_page = 0),
            ),
    );
    controls = controls.child(spacer());
    controls = controls.child(ui::small(
        m,
        format!("登録 {} / 232", doc.compendium_count()),
    ));

    let ids = s.compendium_ids();
    let page_size = 64usize;
    let pages = ids.len().div_ceil(page_size).max(1);
    let current_page = s.compendium_page.min(pages.saturating_sub(1));
    let start = current_page * page_size;
    let end = (start + page_size).min(ids.len());

    let mut grid = ui::card(m).gap(m.x(3.0));
    let mut offset = start;
    while offset < end {
        let mut line = row().w_full().gap(m.x(3.0));
        for _ in 0..4 {
            if offset >= end {
                break;
            }
            let id = ids[offset];
            line = line.child(compendium_cell(s, m, id));
            offset += 1;
        }
        grid = grid.child(line);
    }
    if ids.is_empty() {
        grid = grid.child(ui::muted(m, "該当するペルソナがありません。"));
    }

    let mut pager = row().w_full().gap(m.x(8.0)).center_y();
    pager = pager.child(ui::ghost(m, "前へ").on_press(move |s: &mut Editor| {
        s.compendium_page = current_page.saturating_sub(1);
    }));
    pager = pager.child(ui::small(m, format!("{} / {}", current_page + 1, pages)));
    pager = pager.child(ui::ghost(m, "次へ").on_press(move |s: &mut Editor| {
        s.compendium_page = (current_page + 1).min(pages - 1);
    }));
    pager = pager.child(spacer());
    pager = pager.child(ui::small(m, "クリックで登録を切り替えます。"));

    super::page_shell(m)
        .child(header)
        .child(controls)
        .child(grid)
        .child(pager)
}

fn compendium_cell(s: &Editor, m: &Metrics, id: u16) -> El {
    let registered = s
        .doc
        .as_ref()
        .map(|doc| doc.compendium_registered(id))
        .unwrap_or(false);
    row()
        .grow(1.0)
        .min_w(0.0)
        .h(m.x(28.0))
        .padding(m.x(5.0))
        .gap(m.x(6.0))
        .center_y()
        .radius(m.x(4.0))
        .hover(ui::theme().hover)
        .cursor(Cursor::Pointer)
        .on_press(move |s: &mut Editor| {
            let current = s
                .doc
                .as_ref()
                .map(|doc| doc.compendium_registered(id))
                .unwrap_or(false);
            s.set_compendium(id, !current);
        })
        .child(ui::checkbox(m, registered))
        .child(
            text(format!("{id:03}"))
                .font(ui::mono(m))
                .color(ui::theme().text_secondary)
                .w(m.x(30.0)),
        )
        .child(
            text(tables().persona_name(id))
                .font(m.label())
                .color(if registered {
                    ui::theme().text
                } else {
                    ui::theme().text_secondary
                })
                .grow(1.0)
                .truncate()
                .help_when_truncated(tables().persona_name(id)),
        )
}
