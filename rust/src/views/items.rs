use rune::{row, spacer, text, Cursor, El};

use crate::app::Editor;
use crate::tables::{item_prefix, tables};
use crate::ui::{self, Metrics};

pub fn view(s: &Editor, m: &Metrics) -> El {
    if s.doc.is_none() {
        return ui::empty_state(m, "DATA01〜DATA16 のいずれかを読み込んでください。");
    }
    let categories = tables().categories.clone();
    let category_index = s.category_index();
    let category = &categories[category_index];
    let mut header = super::header_with_actions(
        m,
        "所持品",
        "装備・消耗品・スキルカードなどを編集します。キーアイテムは閲覧のみです。",
    );
    if s.advanced() && !category.read_only {
        header = header
            .child(
                ui::danger_button(m, "一括所持 / 最大").on_press(|s: &mut Editor| s.max_category()),
            )
            .child(ui::danger_button(m, "一括解除").on_press(|s: &mut Editor| s.clear_category()));
    }

    let mut tabs = row().w_full().gap(m.x(4.0)).flex_wrap();
    for (index, item) in categories.iter().enumerate() {
        let label = item.label.clone();
        tabs = tabs.child(
            ui::chip(m, label, index == category_index, move |s: &mut Editor| {
                s.set_category(index)
            })
            .key(format!("item-cat-{index}")),
        );
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
                rune::text_input(&s.item_search, m.body())
                    .single_line()
                    .grow(1.0)
                    .h(m.x(24.0))
                    .on_edit(|s: &mut Editor| &mut s.item_search)
                    .on_input_change(|s: &mut Editor, _| s.item_page = 0),
            ),
    );
    controls = controls.child(
        row()
            .h(m.x(26.0))
            .px(m.x(7.0))
            .gap(m.x(6.0))
            .center_y()
            .radius(m.x(5.0))
            .fill(ui::theme().control_track)
            .hover(ui::theme().hover)
            .cursor(Cursor::Pointer)
            .on_press(|s: &mut Editor| {
                s.owned_only = !s.owned_only;
                s.item_page = 0;
            })
            .child(ui::checkbox(m, s.owned_only))
            .child(ui::small(m, "所持のみ")),
    );
    controls = controls.child(spacer());
    controls = controls.child(ui::small(
        m,
        format!("{} / {} 件", s.item_indices().len(), category.count),
    ));

    let indices = s.item_indices();
    let page_size = 40usize;
    let pages = indices.len().div_ceil(page_size).max(1);
    let current_page = s.item_page.min(pages.saturating_sub(1));
    let start = current_page * page_size;
    let end = (start + page_size).min(indices.len());

    let mut grid = row().w_full().gap(m.x(4.0)).flex_wrap();
    for &index in &indices[start..end] {
        grid = grid.child(
            item_row(s, m, category, index)
                .w_frac(0.49)
                .min_w(m.x(340.0)),
        );
    }
    let mut list = ui::card(m).gap(m.x(4.0));
    list = list.child(grid);
    if indices.is_empty() {
        list = list.child(ui::muted(m, "該当するアイテムがありません。"));
    }

    let mut pager = row().w_full().gap(m.x(8.0)).center_y();
    pager = pager.child(ui::ghost(m, "前へ").on_press(move |s: &mut Editor| {
        s.item_page = current_page.saturating_sub(1);
    }));
    pager = pager.child(ui::small(m, format!("{} / {}", current_page + 1, pages)));
    pager = pager.child(ui::ghost(m, "次へ").on_press(move |s: &mut Editor| {
        s.item_page = (current_page + 1).min(pages - 1);
    }));
    pager = pager.child(spacer());
    pager = pager.child(ui::small(
        m,
        format!(
            "表示 {}〜{} / {} 件",
            if indices.is_empty() { 0 } else { start + 1 },
            end,
            indices.len()
        ),
    ));

    super::page_shell(m)
        .child(header)
        .child(tabs)
        .child(controls)
        .child(list)
        .child(pager)
}

fn item_row(
    s: &Editor,
    m: &Metrics,
    category: &crate::tables::ItemCategoryDef,
    index: usize,
) -> El {
    let name = tables().item_name(&category.key, index);
    let item_id = item_prefix(&category.key) + index as u16;
    let value = s
        .doc
        .as_ref()
        .map(|doc| doc.item_value(&category.key, index))
        .unwrap_or(0);
    let mut line = row()
        .w_full()
        .min_h(m.x(30.0))
        .padding(m.x(6.0))
        .gap(m.x(8.0))
        .center_y()
        .radius(m.x(4.0))
        .hover(ui::theme().hover)
        .child(
            text(format!("{item_id:04X}"))
                .font(ui::mono(m))
                .color(ui::theme().text_secondary)
                .w(m.x(42.0)),
        )
        .child(
            text(name)
                .font(m.body())
                .color(ui::theme().text)
                .grow(1.0)
                .truncate(),
        );
    if category.read_only {
        line = line.child(ui::muted(m, value.to_string())).child(ui::tag(
            m,
            "閲覧のみ",
            ui::theme().text_secondary,
        ));
    } else if category.kind == "owned" {
        line = line.child(
            row()
                .h(m.x(22.0))
                .px(m.x(6.0))
                .gap(m.x(5.0))
                .center_y()
                .radius(m.x(4.0))
                .fill(if value > 0 {
                    ui::accent().alpha(0.13)
                } else {
                    ui::theme().control_track
                })
                .cursor(Cursor::Pointer)
                .on_press(move |s: &mut Editor| s.toggle_item(index))
                .child(ui::checkbox(m, value > 0))
                .child(
                    text(if value > 0 { "所持" } else { "未所持" })
                        .font(ui::small_style(m))
                        .color(if value > 0 {
                            ui::accent()
                        } else {
                            ui::theme().text_secondary
                        }),
                ),
        );
    } else if let Some(field) = s.item_fields.get(index) {
        line = line.child(super::field_input(
            m,
            field,
            m.x(64.0),
            s.item_value_input(index).is_some(),
            move |s: &mut Editor| &mut s.item_fields[index].edit,
            move |s: &mut Editor| s.commit_item(index),
        ));
    } else {
        line = line.child(ui::muted(m, value.to_string()));
    }
    line
}
