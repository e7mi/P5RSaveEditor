use rune::{row, spacer, text, El};

use crate::app::{Action, Editor};
use crate::calendar;
use crate::gamedata::SaveDocument;
use crate::input;
use crate::ui::{self, Metrics};

pub fn view(s: &Editor, m: &Metrics) -> El {
    let Some(doc) = &s.doc else {
        return ui::empty_state(m, "DATA01〜DATA16 のいずれかを読み込んでください。");
    };
    let mut page = super::page_shell(m).child(super::header_with_actions(
        m,
        "基本",
        "セーブ情報・所持金・ソーシャルを編集します。",
    ));
    if doc.is_clear_data() {
        page = page.child(super::warning_strip(
            m,
            "クリアデータです。通常の進行データとは内容が異なります。",
        ));
    }
    if !doc.crc_warning.is_empty() {
        page = page.child(super::warning_strip(
            m,
            &format!("{}。保存すると CRC を再計算します。", doc.crc_warning),
        ));
    }
    if s.date_mismatch() {
        page = page.child(super::warning_strip(
            m,
            &format!(
                "ヘッダーの日付 ({}) とセーブ本体の日付 ({}) が一致していません。日付を保存し直すと同期されます。",
                calendar::format_date(doc.header.day),
                calendar::format_date(doc.date().0)
            ),
        ));
    }

    let mut columns = super::two_column(m);
    columns = columns
        .child(info_card(s, doc, m))
        .child(money_card(s, doc, m));
    page = page.child(columns);
    page = page.child(social_card(s, doc, m));
    if s.advanced() && !doc.is_clear_data() {
        page = page.child(date_card(s, doc, m));
    } else if doc.is_clear_data() {
        page = page.child(
            row().w_full().padding(m.x(8.0)).child(
                text("クリアデータの日付は編集できません。")
                    .font(m.label())
                    .color(ui::theme().text_secondary),
            ),
        );
    } else {
        page = page.child(
            row().w_full().padding(m.x(8.0)).child(
                text("日付の変更は高度なモードで表示されます。")
                    .font(m.label())
                    .color(ui::theme().text_secondary),
            ),
        );
    }
    page
}

fn info_card(s: &Editor, doc: &SaveDocument, m: &Metrics) -> El {
    let mut card = super::column_card(m).child(super::card_header(m, "セーブ情報"));
    card = card.child(
        row()
            .gap(m.x(8.0))
            .center_y()
            .child(ui::field_label(m, "名前"))
            .child(super::keyed_field_input(
                m,
                "field-name",
                &s.basic.name,
                m.x(200.0),
                !s.basic.name.text().trim().is_empty(),
                |s: &mut Editor| &mut s.basic.name.edit,
                |s: &mut Editor| s.commit_name(),
            )),
    );
    card = card.child(
        row()
            .gap(m.x(8.0))
            .center_y()
            .child(ui::field_label(m, "プレイ時間"))
            .child(super::keyed_field_input(
                m,
                "field-playtime",
                &s.basic.playtime,
                m.x(92.0),
                s.playtime_value().is_some(),
                |s: &mut Editor| &mut s.basic.playtime.edit,
                |s: &mut Editor| s.commit_playtime(),
            )),
    );
    card = card.child(
        row()
            .gap(m.x(8.0))
            .center_y()
            .child(ui::field_label(m, ""))
            .child(
                text(match s.playtime_value() {
                    Some(minutes) => format!("= {}", input::playtime_text(minutes)),
                    None => "例: 87:09 / 5229 (分)".into(),
                })
                .font(ui::small_style(m))
                .color(ui::theme().text_secondary),
            ),
    );
    card = card.child(
        row()
            .w_full()
            .gap(m.x(8.0))
            .center_y()
            .child(ui::field_label(m, "難易度"))
            .child(
                row()
                    .gap(m.x(4.0))
                    .flex_wrap()
                    .children((0u8..5).map(|value| {
                        let selected = doc.header.difficulty == value;
                        ui::chip(
                            m,
                            input::difficulty_label(value),
                            selected,
                            move |s: &mut Editor| s.set_difficulty(value),
                        )
                    })),
            ),
    );
    card = card.child(
        row()
            .gap(m.x(8.0))
            .center_y()
            .child(ui::field_label(m, "日付"))
            .child(ui::body(m, calendar::format_date(doc.header.day)))
            .child(ui::muted(m, calendar::time_label(doc.header.time))),
    );
    card.child(
        row()
            .gap(m.x(8.0))
            .center_y()
            .child(ui::field_label(m, "主人公Lv"))
            .child(ui::body(m, doc.header.level.to_string())),
    )
}

fn money_card(s: &Editor, doc: &SaveDocument, m: &Metrics) -> El {
    let mut card = super::column_card(m).child(super::card_header(m, "所持金"));
    card = card.child(
        row()
            .gap(m.x(8.0))
            .center_y()
            .child(ui::field_label(m, "金額"))
            .child(super::keyed_field_input(
                m,
                "field-money",
                &s.basic.money,
                m.x(150.0),
                s.money_value().is_some(),
                |s: &mut Editor| &mut s.basic.money.edit,
                |s: &mut Editor| s.commit_money(),
            ))
            .child(ui::primary(m, "MAX").on_press(|s: &mut Editor| {
                s.request_confirm(
                    Action::MaxMoney,
                    "所持金 MAX",
                    "所持金を 9,999,999 にします。実行しますか？",
                )
            })),
    );
    card = card.child(
        row()
            .gap(m.x(8.0))
            .child(ui::field_label(m, ""))
            .child(
                text(match s.money_value() {
                    Some(value) => format!("= {}", input::money_text(value)),
                    None => "0〜9,999,999".into(),
                })
                .font(ui::small_style(m))
                .color(ui::theme().text_secondary),
            )
            .child(spacer())
            .child(ui::muted(
                m,
                format!("現在 {}", input::money_text(doc.money())),
            )),
    );
    card.child(ui::small(
        m,
        "変更は「保存」を押すまでファイルに書き込まれません。",
    ))
}

fn social_card(s: &Editor, doc: &SaveDocument, m: &Metrics) -> El {
    let stats = doc.social_stats();
    let mut card = ui::card(m);
    let mut header = super::card_header(m, "ソーシャル");
    if s.advanced() {
        header = header.child(ui::danger_button(m, "全MAX").on_press(|s: &mut Editor| {
            s.request_confirm(
                Action::MaxSocial,
                "ソーシャル MAX",
                "5 項目を MAX にします。実行しますか？",
            )
        }));
    }
    card = card.child(header);
    let mut grid = row().w_full().gap(m.x(8.0)).flex_wrap();
    for stat in &stats {
        let index = stat.index;
        let mut name_col = rune::col().w(m.x(92.0)).gap(m.x(1.0));
        name_col = name_col
            .child(ui::body(m, &stat.name))
            .child(ui::small(m, format!("ランク {}", stat.rank)));
        let mut line = row()
            .w_frac(0.49)
            .min_w(m.x(360.0))
            .min_h(m.x(38.0))
            .padding(m.x(6.0))
            .gap(m.x(8.0))
            .center_y()
            .radius(m.x(4.0))
            .hover(ui::theme().hover)
            .child(name_col)
            .child(
                text("★".repeat(stat.rank as usize))
                    .font(m.body())
                    .color(ui::accent()),
            )
            .child(super::field_input(
                m,
                &s.basic.social[index],
                m.x(70.0),
                s.social_value(index).is_some(),
                move |s: &mut Editor| &mut s.basic.social[index].edit,
                move |s: &mut Editor| s.commit_social(index),
            ))
            .child(
                text(match stat.next {
                    Some(next) => format!("次 {next}"),
                    None => "上限".into(),
                })
                .font(ui::small_style(m))
                .color(ui::theme().text_secondary)
                .w(m.x(52.0)),
            );
        if s.advanced() {
            line = line
                .child(ui::link(m, "MAX").on_press(move |s: &mut Editor| s.max_social_stat(index)));
        }
        grid = grid.child(line);
    }
    card = card.child(grid);
    card.child(ui::small(m, "ポイントはゲーム内のランク計算に使われます。"))
}

fn date_card(s: &Editor, doc: &SaveDocument, m: &Metrics) -> El {
    let mut card = ui::card(m);
    card = card.child(super::card_header(m, "日付と時間帯"));
    card = card.child(
        row()
            .gap(m.x(10.0))
            .center_y()
            .child(ui::field_label(m, "日付"))
            .child(super::keyed_field_input(
                m,
                "field-date",
                &s.basic.date,
                m.x(92.0),
                s.date_value().is_some(),
                |s: &mut Editor| &mut s.basic.date.edit,
                |s: &mut Editor| s.commit_date(),
            ))
            .child(
                text(match s.date_value() {
                    Some(day) => format!(
                        "{} {}",
                        calendar::format_date(day),
                        calendar::time_label(s.date_time)
                    ),
                    None => "例: 12/24".into(),
                })
                .font(ui::small_style(m))
                .color(ui::theme().text_secondary),
            )
            .child(spacer())
            .child(ui::small(
                m,
                format!("保存値 {}", calendar::format_date(doc.date().0)),
            )),
    );
    let mut times = row().w_full().gap(m.x(4.0)).flex_wrap();
    for (value, label) in calendar::TIMES {
        times = times.child(
            ui::chip(m, label, s.date_time == value, move |s: &mut Editor| {
                s.select_time(value)
            })
            .key(format!("time-{value}")),
        );
    }
    card = card.child(times);
    card.child(super::danger_strip(
        m,
        "日付の変更は危険です。ストーリー進行・イベント予定・コープの解放時期と矛盾する可能性があります。必ずバックアップを取ってください。",
    ))
}
