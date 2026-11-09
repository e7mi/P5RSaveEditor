use rune::{col, row, spacer, text, El};

use crate::app::{Action, Editor};
use crate::ui::{self, Metrics};

pub fn view(s: &Editor, m: &Metrics) -> El {
    let Some(doc) = &s.doc else {
        return ui::empty_state(m, "DATA01〜DATA16 のいずれかを読み込んでください。");
    };
    let filter = s.confidant_filter();
    let states: Vec<_> = doc
        .confidants()
        .into_iter()
        .filter(|state| {
            filter.is_empty()
                || state.def.name.to_lowercase().contains(&filter)
                || state.def.arcana.to_lowercase().contains(&filter)
        })
        .collect();

    let mut header =
        super::header_with_actions(m, "コープ", "ランクとポイントを個別に編集します。");
    if s.advanced() {
        header = header.child(ui::danger_button(m, "全MAX").on_press(|s: &mut Editor| {
            s.request_confirm(
                Action::MaxConfidants,
                "コープ MAX",
                "23 件をすべてランク 10 にします。実行しますか？",
            )
        }));
    }

    let search = row()
        .h(m.x(26.0))
        .w(m.x(240.0))
        .px(m.x(7.0))
        .gap(m.x(6.0))
        .center_y()
        .radius(m.x(5.0))
        .fill(ui::theme().control_track)
        .border(1.0, ui::theme().card_border)
        .child(ui::small(m, "検索"))
        .child(
            rune::text_input(&s.confidant_search, m.body())
                .single_line()
                .grow(1.0)
                .h(m.x(24.0))
                .on_edit(|s: &mut Editor| &mut s.confidant_search),
        );

    let mut page = super::page_shell(m).child(header).child(
        row()
            .w_full()
            .gap(m.x(10.0))
            .center_y()
            .child(search)
            .child(spacer())
            .child(ui::small(m, format!("{} / 23 件", states.len()))),
    );

    let mut grid = row().w_full().gap(m.x(6.0)).flex_wrap();
    for state in &states {
        grid = grid.child(confidant_card(s, m, state));
    }
    let mut body = ui::card(m).gap(m.x(6.0));
    if states.is_empty() {
        body = body.child(ui::muted(m, "該当するコープがありません。"));
    } else {
        body = body.child(grid);
    }
    page = page.child(body);
    page.child(ui::small(
        m,
        "ランクを下げても通過済みイベントは戻りません。ストーリー固定のコープは表示のみ有効な場合があります。",
    ))
}

fn confidant_card(s: &Editor, m: &Metrics, state: &crate::gamedata::ConfidantView) -> El {
    let t = ui::theme();
    let index = state.index;
    let top = row().w_full().gap(m.x(6.0)).center_y().child(
        text(&state.def.name)
            .font(m.body())
            .color(t.text)
            .grow(1.0)
            .wrap(),
    );
    let mut meta = row().w_full().gap(m.x(4.0)).center_y();
    meta = meta.child(ui::small(m, &state.def.arcana));
    if state.def.story_locked {
        meta = meta.child(ui::tag(m, "ストーリー", ui::warning()));
    }
    if !state.started() {
        meta = meta.child(ui::tag(m, "未開始", t.text_secondary));
    } else if state.rank >= 10 {
        meta = meta.child(ui::tag(m, "MAX", ui::success()));
    }
    meta = meta
        .child(spacer())
        .child(ui::small(m, "ランク"))
        .child(super::field_input(
            m,
            &s.confidants.rank[index],
            m.x(52.0),
            s.confidant_rank_value(index).is_some(),
            move |s: &mut Editor| &mut s.confidants.rank[index].edit,
            move |s: &mut Editor| s.commit_confidant_rank(index),
        ))
        .child(ui::small(m, "ポイント"))
        .child(super::field_input(
            m,
            &s.confidants.points[index],
            m.x(60.0),
            s.confidant_points_value(index).is_some(),
            move |s: &mut Editor| &mut s.confidants.points[index].edit,
            move |s: &mut Editor| s.commit_confidant_points(index),
        ));
    col()
        .w_frac(0.49)
        .min_w(m.x(340.0))
        .padding(m.x(7.0))
        .gap(m.x(4.0))
        .radius(m.x(5.0))
        .fill(t.bg)
        .border(1.0, t.card_border)
        .hover(t.hover)
        .child(top)
        .child(meta)
}
