use rune::tok::type_;
use rune::{col, el, row, spacer, text, Color, Cursor, El, ScrollSpec};

use crate::app::{Editor, Modal, PickerKind, PickerTarget, SlotSummary};
use crate::tables::tables;
use crate::ui::{self, Metrics};

pub fn modal(s: &Editor, m: &Metrics) -> Option<El> {
    let body = match s.modal.as_ref()? {
        Modal::Slots => slots_modal(s, m),
        Modal::Confirm => confirm_modal(s, m),
        Modal::Message(title, message) => message_modal(m, title, message),
        Modal::Picker => picker_modal(s, m),
        Modal::Persona => persona_modal(s, m),
    };
    Some(
        el().absolute(0.0, 0.0)
            .w_full()
            .h_full()
            .fill(Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.66,
            })
            .center()
            .modal(true, |s: &mut Editor| {
                if !matches!(s.modal, Some(Modal::Persona)) {
                    s.close_modal();
                }
            })
            .child(body),
    )
}

fn modal_card(m: &Metrics, width_frac: f32, max_width: f32) -> El {
    let t = ui::theme();
    col()
        .w_frac(width_frac)
        .max_w(max_width)
        .max_h(m.height - m.x(48.0))
        .padding(m.x(14.0))
        .gap(m.x(10.0))
        .radius(m.x(10.0))
        .fill(t.card)
        .border(1.0, t.card_border)
        .shadow(
            m.x(8.0),
            m.x(30.0),
            Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.55,
            },
        )
}

fn slots_modal(s: &Editor, m: &Metrics) -> El {
    let current = s.doc.as_ref().map(|doc| doc.path.clone());
    let mut card = modal_card(m, 0.72, m.x(760.0));
    card = card
        .child(ui::title(m, "セーブスロット"))
        .child(ui::small(m, "読み込む DATA01〜DATA16 を選択してください。"))
        .child(ui::divider());
    let mut list = col().w_full().gap(m.x(1.0));
    for slot in &s.slots {
        list = list.child(slot_row(m, slot, current.as_ref()));
    }
    card = card.child(
        col()
            .w_full()
            .h((m.height * 0.52).max(m.x(220.0)))
            .scroll(ScrollSpec {
                stick_bottom: false,
            })
            .child(list),
    );
    card.child(
        row()
            .w_full()
            .gap(m.x(8.0))
            .child(spacer())
            .child(ui::ghost(m, "閉じる").on_press(|s: &mut Editor| s.close_modal())),
    )
}

fn slot_row(m: &Metrics, slot: &SlotSummary, current: Option<&std::path::PathBuf>) -> El {
    let t = ui::theme();
    let is_current = current == Some(&slot.path);
    let path = slot.path.clone();
    row()
        .w_full()
        .min_h(m.x(34.0))
        .padding(m.x(6.0))
        .gap(m.x(8.0))
        .center_y()
        .radius(m.x(5.0))
        .fill(if is_current {
            ui::accent().alpha(0.12)
        } else {
            Color::TRANSPARENT
        })
        .hover(t.hover)
        .cursor(Cursor::Pointer)
        .on_press(move |s: &mut Editor| s.request_load_slot(path.clone()))
        .child(
            text(&slot.label)
                .font(if is_current {
                    ui::Metrics::font(m, type_::TITLE)
                } else {
                    m.body()
                })
                .color(if slot.valid {
                    if is_current {
                        ui::accent()
                    } else {
                        t.text
                    }
                } else {
                    t.text_secondary
                })
                .truncate(),
        )
        .child(spacer())
        .child(if slot.valid {
            ui::tag(m, "読込可", ui::success())
        } else {
            ui::tag(m, "不可", t.text_secondary)
        })
}

fn confirm_modal(s: &Editor, m: &Metrics) -> El {
    modal_card(m, 0.55, m.x(520.0))
        .child(ui::title(m, &s.confirm_title))
        .child(ui::body(m, &s.confirm_message))
        .child(
            row()
                .w_full()
                .gap(m.x(8.0))
                .child(spacer())
                .child(
                    ui::ghost(m, "キャンセル")
                        .key("confirm-cancel")
                        .on_press(|s: &mut Editor| s.confirm_cancel()),
                )
                .child(
                    ui::primary(m, "実行")
                        .key("confirm-ok")
                        .on_press(|s: &mut Editor| s.confirm_ok()),
                ),
        )
}

fn message_modal(m: &Metrics, title: &str, message: &str) -> El {
    modal_card(m, 0.55, m.x(560.0))
        .child(ui::title(m, title))
        .child(ui::body(m, message).wrap())
        .child(
            row()
                .w_full()
                .child(spacer())
                .child(ui::primary(m, "OK").on_press(|s: &mut Editor| s.close_modal())),
        )
}

fn picker_modal(s: &Editor, m: &Metrics) -> El {
    let Some(picker) = &s.picker else {
        return message_modal(m, "エラー", "選択画面の状態がありません。");
    };
    let title = match picker.kind {
        PickerKind::Persona => "ペルソナを選択",
        PickerKind::Skill => "スキルを選択",
        PickerKind::Trait => "特性を選択",
    };
    let rows = s.picker_rows();
    let page_size = 60usize;
    let pages = rows.len().div_ceil(page_size).max(1);
    let current_page = picker.page.min(pages.saturating_sub(1));
    let start = current_page * page_size;
    let end = (start + page_size).min(rows.len());

    let mut card = modal_card(m, 0.78, m.x(720.0));
    card = card
        .child(ui::title(m, title))
        .child(
            row()
                .w_full()
                .h(m.x(28.0))
                .px(m.x(7.0))
                .gap(m.x(6.0))
                .center_y()
                .radius(m.x(5.0))
                .fill(ui::theme().control_track)
                .border(1.0, ui::theme().card_border)
                .child(ui::small(m, "検索"))
                .child(
                    rune::text_input(&picker.search, m.body())
                        .key("picker-search")
                        .single_line()
                        .grow(1.0)
                        .h(m.x(24.0))
                        .on_edit(|s: &mut Editor| &mut s.picker.as_mut().unwrap().search)
                        .on_input_change(|s: &mut Editor, _| {
                            if let Some(picker) = &mut s.picker {
                                picker.page = 0;
                            }
                        }),
                ),
        )
        .child(ui::small(m, format!("{} 件", rows.len())));
    let mut list = col().w_full().gap(m.x(1.0));
    for (id, name) in &rows[start..end] {
        let t = ui::theme();
        let id = *id;
        list = list.child(
            row()
                .w_full()
                .min_h(m.x(26.0))
                .px(m.x(6.0))
                .gap(m.x(8.0))
                .center_y()
                .radius(m.x(4.0))
                .hover(t.hover)
                .cursor(Cursor::Pointer)
                .on_press(move |s: &mut Editor| s.picker_select(id))
                .child(
                    text(format!("{id:04}"))
                        .font(ui::mono(m))
                        .color(t.text_secondary)
                        .w(m.x(42.0)),
                )
                .child(text(name).font(m.body()).color(t.text).truncate()),
        );
    }
    card = card.child(
        col()
            .w_full()
            .h((m.height * 0.44).max(m.x(200.0)))
            .scroll(ScrollSpec {
                stick_bottom: false,
            })
            .child(list),
    );
    card.child(
        row()
            .w_full()
            .gap(m.x(8.0))
            .center_y()
            .child(ui::ghost(m, "前へ").on_press(move |s: &mut Editor| {
                if let Some(picker) = &mut s.picker {
                    picker.page = current_page.saturating_sub(1);
                }
            }))
            .child(ui::small(m, format!("{} / {}", current_page + 1, pages)))
            .child(ui::ghost(m, "次へ").on_press(move |s: &mut Editor| {
                if let Some(picker) = &mut s.picker {
                    picker.page = (current_page + 1).min(pages - 1);
                }
            }))
            .child(spacer())
            .child(ui::ghost(m, "戻る").on_press(|s: &mut Editor| {
                s.picker = None;
                s.modal = Some(Modal::Persona);
            })),
    )
}

fn persona_modal(s: &Editor, m: &Metrics) -> El {
    let Some(editor) = &s.persona else {
        return message_modal(m, "エラー", "編集対象のペルソナがありません。");
    };
    if s.doc.is_none() {
        return message_modal(m, "エラー", "セーブが読み込まれていません。");
    }
    let name = tables().persona_name(editor.draft.id);
    let trait_name = tables().trait_name(editor.draft.trait_id);
    let character_name = tables()
        .characters
        .get(editor.character)
        .map(|character| character.name.clone())
        .unwrap_or_default();
    let mut card = modal_card(m, 0.9, m.x(820.0));
    card = card
        .child(
            row()
                .w_full()
                .center_y()
                .child(ui::title(
                    m,
                    format!(
                        "ペルソナ編集  /  {}  /  枠 {:02}",
                        character_name,
                        editor.slot + 1
                    ),
                ))
                .child(spacer())
                .child(ui::small(m, format!("ID {}  {}", editor.draft.id, name))),
        )
        .child(ui::divider());

    let mut body = col().w_full().gap(m.x(8.0));
    let mut id_row = row().w_full().gap(m.x(8.0)).center_y();
    id_row = id_row
        .child(ui::field_label(m, "ペルソナ"))
        .child(ui::body(m, format!("{}  {}", editor.draft.id, name)))
        .child(spacer());
    if !editor.id_locked {
        id_row = id_row.child(ui::ghost(m, "選択").on_press(|s: &mut Editor| {
            s.open_picker(PickerKind::Persona, PickerTarget::PersonaId)
        }));
    }
    body = body.child(id_row);

    let mut numbers = row().w_full().gap(m.x(10.0)).flex_wrap();
    numbers = numbers
        .child(super::labelled_field(
            m,
            "レベル",
            &editor.level,
            m.x(62.0),
            persona_level_valid(s),
            |s: &mut Editor| &mut s.persona.as_mut().unwrap().level.edit,
            |s: &mut Editor| s.commit_persona_drafts(),
        ))
        .child(super::labelled_field(
            m,
            "EXP",
            &editor.xp,
            m.x(96.0),
            persona_xp_valid(s),
            |s: &mut Editor| &mut s.persona.as_mut().unwrap().xp.edit,
            |s: &mut Editor| s.commit_persona_drafts(),
        ));
    body = body.child(numbers);

    let mut trait_row = row().w_full().gap(m.x(8.0)).center_y();
    trait_row = trait_row
        .child(ui::field_label(m, "特性"))
        .child(ui::body(m, trait_name))
        .child(spacer())
        .child(
            ui::ghost(m, "選択")
                .on_press(|s: &mut Editor| s.open_picker(PickerKind::Trait, PickerTarget::Trait)),
        );
    body = body.child(trait_row);

    body = body.child(ui::section(m, "ステータス"));
    let labels = ["力", "魔", "耐", "速", "運"];
    let mut stats = row().w_full().gap(m.x(8.0)).flex_wrap();
    for (index, label) in labels.iter().enumerate() {
        stats = stats.child(super::labelled_field(
            m,
            label,
            &editor.stats[index],
            m.x(54.0),
            persona_stat_valid(s, index),
            move |s: &mut Editor| &mut s.persona.as_mut().unwrap().stats[index].edit,
            |s: &mut Editor| s.commit_persona_drafts(),
        ));
    }
    body = body.child(stats);

    body = body.child(ui::section(m, "スキル"));
    let mut skills = col().w_full().gap(m.x(1.0));
    for slot in 0..8 {
        let skill_id = editor.draft.skills[slot];
        let skill_name = tables().skill_name(skill_id);
        skills = skills.child(
            row()
                .w_full()
                .min_h(m.x(26.0))
                .gap(m.x(8.0))
                .center_y()
                .child(
                    text(format!("{:02}", slot + 1))
                        .font(ui::mono(m))
                        .color(ui::theme().text_secondary)
                        .w(m.x(22.0)),
                )
                .child(
                    text(skill_name)
                        .font(m.body())
                        .color(ui::theme().text)
                        .grow(1.0)
                        .truncate(),
                )
                .child(ui::link(m, "変更").on_press(move |s: &mut Editor| {
                    s.open_picker(PickerKind::Skill, PickerTarget::Skill(slot))
                })),
        );
    }
    body = body.child(skills);

    if s.advanced() && editor.can_clear {
        let mut clear_row = row().w_full().gap(m.x(8.0)).center_y().child(
            ui::danger_button(
                m,
                if editor.draft.clear {
                    "空にするのをやめる"
                } else {
                    "このスロットを空にする"
                },
            )
            .on_press(|s: &mut Editor| s.mark_persona_clear()),
        );
        if editor.draft.clear {
            clear_row = clear_row.child(ui::tag(m, "適用で空になります", ui::danger()));
        }
        body = body.child(ui::divider()).child(clear_row);
    }
    card = card.child(
        col()
            .w_full()
            .max_h(m.height - m.x(170.0))
            .scroll(ScrollSpec {
                stick_bottom: false,
            })
            .child(body),
    );
    card.child(
        row()
            .w_full()
            .gap(m.x(8.0))
            .child(spacer())
            .child(ui::ghost(m, "キャンセル").on_press(|s: &mut Editor| s.close_modal()))
            .child(ui::primary(m, "適用").on_press(|s: &mut Editor| s.apply_persona())),
    )
}

fn persona_level_valid(s: &Editor) -> bool {
    s.persona
        .as_ref()
        .and_then(|persona| crate::input::parse_u8(persona.level.text()))
        .map(|value| (1..=99).contains(&value))
        .unwrap_or(false)
}

fn persona_xp_valid(s: &Editor) -> bool {
    s.persona
        .as_ref()
        .and_then(|persona| crate::input::parse_u32(persona.xp.text()))
        .map(|value| value <= 2_000_000_000)
        .unwrap_or(false)
}

fn persona_stat_valid(s: &Editor, index: usize) -> bool {
    s.persona
        .as_ref()
        .and_then(|persona| crate::input::parse_u8(persona.stats[index].text()))
        .map(|value| value <= 99)
        .unwrap_or(false)
}
