use rune::{row, spacer, text, Color, Cursor, El};

use crate::app::Editor;
use crate::gamedata::{PersonaView, SaveDocument};
use crate::tables::tables;
use crate::ui::{self, Metrics};

pub fn view(s: &Editor, m: &Metrics) -> El {
    let Some(doc) = &s.doc else {
        return ui::empty_state(m, "DATA01〜DATA16 のいずれかを読み込んでください。");
    };
    let characters = doc.characters();
    let index = s.selected_character.min(characters.len().saturating_sub(1));
    let joker = tables()
        .characters
        .get(index)
        .map(|definition| definition.joker)
        .unwrap_or(false);

    super::page_shell(m)
        .child(super::header_with_actions(
            m,
            "メンバー",
            "HP・SP・レベル・EXP とペルソナを 1 画面で編集します。",
        ))
        .child(
            row()
                .w_full()
                .gap(m.x(12.0))
                .items_start()
                .child(character_list(m, &characters, index))
                .child(detail(s, doc, m, index, joker)),
        )
}

fn character_list(
    m: &Metrics,
    characters: &[crate::gamedata::CharacterView],
    selected: usize,
) -> El {
    let mut list = ui::card(m).w(m.x(196.0)).gap(m.x(1.0));
    for character in characters {
        let index = character.index;
        let active = index == selected;
        list = list.child(
            row()
                .w_full()
                .min_h(m.x(38.0))
                .px(m.x(7.0))
                .gap(m.x(6.0))
                .center_y()
                .radius(m.x(4.0))
                .fill(if active {
                    ui::accent().alpha(0.14)
                } else {
                    Color::TRANSPARENT
                })
                .hover(ui::theme().hover)
                .cursor(Cursor::Pointer)
                .on_press(move |s: &mut Editor| s.selected_character = index)
                .child(
                    rune::col()
                        .grow(1.0)
                        .gap(m.x(1.0))
                        .child(text(&character.name).font(m.body()).color(if active {
                            ui::accent()
                        } else {
                            ui::theme().text
                        }))
                        .child(ui::small(
                            m,
                            format!(
                                "Lv {}  HP {}  SP {}",
                                character.level, character.hp, character.sp
                            ),
                        )),
                ),
        );
    }
    list
}

fn detail(s: &Editor, doc: &SaveDocument, m: &Metrics, index: usize, joker: bool) -> El {
    let characters = doc.characters();
    let character = &characters[index];
    let mut col = rune::col().grow(1.0).min_w(0.0).gap(m.x(12.0));

    let mut stats = ui::card(m);
    let mut title_row = row()
        .w_full()
        .center_y()
        .child(ui::title(m, &character.name))
        .child(spacer());
    title_row = title_row.child(if joker {
        ui::tag(m, "主人公", ui::accent())
    } else {
        ui::tag(m, "メンバー", ui::theme().text_secondary)
    });
    stats = stats.child(title_row);
    let mut fields = row().w_full().gap(m.x(10.0)).flex_wrap();
    let drafts = s.members.get(index);
    if let Some(drafts) = drafts {
        fields = fields
            .child(super::labelled_field(
                m,
                "HP",
                &drafts.hp,
                m.x(78.0),
                s.member_hp_value(index).is_some(),
                move |s: &mut Editor| &mut s.members[index].hp.edit,
                move |s: &mut Editor| s.commit_member(index),
            ))
            .child(super::labelled_field(
                m,
                "SP",
                &drafts.sp,
                m.x(78.0),
                s.member_sp_value(index).is_some(),
                move |s: &mut Editor| &mut s.members[index].sp.edit,
                move |s: &mut Editor| s.commit_member(index),
            ))
            .child(super::labelled_field(
                m,
                "レベル",
                &drafts.level,
                m.x(64.0),
                s.member_level_value(index).is_some(),
                move |s: &mut Editor| &mut s.members[index].level.edit,
                move |s: &mut Editor| s.commit_member(index),
            ))
            .child(super::labelled_field(
                m,
                "EXP",
                &drafts.xp,
                m.x(96.0),
                s.member_xp_value(index).is_some(),
                move |s: &mut Editor| &mut s.members[index].xp.edit,
                move |s: &mut Editor| s.commit_member(index),
            ));
    }
    stats = stats.child(fields);
    stats = stats.child(ui::small(
        m,
        if joker {
            "主人公のレベルと EXP はキャラクター側の値です。ペルソナのレベルとは別です。"
        } else {
            "メンバーのレベルと EXP は専用ペルソナの値です。"
        },
    ));
    col = col.child(stats);

    let personas = doc.personas(index);
    let slots = if joker { 12 } else { 1 };
    let mut table = ui::card(m).gap(m.x(0.0));
    let mut header = super::card_header(m, "ペルソナ");
    header = header.child(ui::small(
        m,
        if joker {
            "スロット 01 が装備中"
        } else {
            "専用ペルソナ"
        },
    ));
    table = table.child(header);
    table = table.child(persona_header(m));
    table = table.child(ui::divider());
    for slot in 0..slots {
        if let Some(persona) = personas.get(slot) {
            table = table.child(persona_row(m, index, persona, joker));
        }
    }
    col = col.child(table);
    col
}

fn persona_header(m: &Metrics) -> El {
    row()
        .w_full()
        .padding(m.x(6.0))
        .gap(m.x(8.0))
        .child(
            text("枠")
                .font(ui::small_style(m))
                .color(ui::theme().text_secondary)
                .w(m.x(24.0)),
        )
        .child(
            text("ペルソナ")
                .font(ui::small_style(m))
                .color(ui::theme().text_secondary)
                .grow(1.0),
        )
        .child(
            text("Lv")
                .font(ui::small_style(m))
                .color(ui::theme().text_secondary)
                .w(m.x(34.0)),
        )
        .child(
            text("力/魔/耐/速/運")
                .font(ui::small_style(m))
                .color(ui::theme().text_secondary)
                .w(m.x(118.0)),
        )
        .child(
            text("スキル")
                .font(ui::small_style(m))
                .color(ui::theme().text_secondary)
                .grow(1.2),
        )
        .child(text("").font(ui::small_style(m)).w(m.x(46.0)))
}

fn persona_row(m: &Metrics, character: usize, persona: &PersonaView, joker: bool) -> El {
    let slot = persona.slot;
    let equipped = joker && slot == 0 && !persona.empty;
    let skills: Vec<String> = persona
        .skills
        .iter()
        .filter(|&&skill| skill != 0)
        .take(4)
        .map(|&skill| tables().skill_name(skill))
        .collect();
    let skills_text = if skills.is_empty() {
        "なし".to_string()
    } else {
        skills.join(" / ")
    };
    row()
        .w_full()
        .min_h(m.x(36.0))
        .padding(m.x(6.0))
        .gap(m.x(8.0))
        .center_y()
        .radius(m.x(4.0))
        .hover(ui::theme().hover)
        .child(
            text(format!("{:02}", slot + 1))
                .font(ui::small_style(m))
                .color(ui::theme().text_secondary)
                .w(m.x(24.0)),
        )
        .child(
            rune::col()
                .grow(1.0)
                .min_w(m.x(150.0))
                .gap(m.x(1.0))
                .child(
                    text(if persona.empty {
                        "空きスロット".to_string()
                    } else {
                        persona.name.clone()
                    })
                    .font(m.body())
                    .color(if persona.empty {
                        ui::theme().text_secondary
                    } else {
                        ui::theme().text
                    })
                    .truncate(),
                )
                .child(
                    rune::row()
                        .gap(m.x(5.0))
                        .center_y()
                        .child(ui::small(m, format!("ID {}", persona.id)))
                        .child(if equipped {
                            ui::tag(m, "装備中", ui::accent())
                        } else if slot == 0 && joker {
                            ui::tag(m, "先頭", ui::theme().text_secondary)
                        } else {
                            spacer()
                        }),
                ),
        )
        .child(
            text(if persona.empty {
                "-".into()
            } else {
                persona.level.to_string()
            })
            .font(m.body())
            .color(ui::theme().text)
            .w(m.x(34.0)),
        )
        .child(
            text(if persona.empty {
                "-".into()
            } else {
                format!(
                    "{}/{}/{}/{}/{}",
                    persona.stats[0],
                    persona.stats[1],
                    persona.stats[2],
                    persona.stats[3],
                    persona.stats[4]
                )
            })
            .font(ui::small_style(m))
            .color(ui::theme().text_secondary)
            .w(m.x(118.0)),
        )
        .child(
            text(skills_text)
                .font(ui::small_style(m))
                .color(ui::theme().text_secondary)
                .grow(1.2)
                .truncate()
                .help_when_truncated(
                    persona
                        .skills
                        .iter()
                        .filter(|&&skill| skill != 0)
                        .map(|&skill| tables().skill_name(skill))
                        .collect::<Vec<_>>()
                        .join(" / "),
                ),
        )
        .child(
            ui::link(m, if persona.empty { "作成" } else { "編集" })
                .on_press(move |s: &mut Editor| s.open_persona(character, slot)),
        )
}
