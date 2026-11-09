use std::path::{Path, PathBuf};

use p5r_save_editor::gamedata::{PersonaEdit, SaveDocument, NAME_OFFSET};
use p5r_save_editor::savefile::{SaveContainer, DATA_SIZE};
use p5r_save_editor::{calendar, input};

// Real saves are not distributed. Set P5R_TEST_SAVE to run this check locally.
#[test]
fn real_save_reads_and_round_trips() {
    let Ok(path) = std::env::var("P5R_TEST_SAVE") else {
        eprintln!("P5R_TEST_SAVE not set; skipping");
        return;
    };
    let mut document = SaveDocument::load(Path::new(&path)).expect("load real save");
    assert_eq!(document.data.len(), DATA_SIZE);
    assert!(document.container.file_crc_ok);
    assert!(document.container.data_crc_ok);
    assert!(document.money() <= 9_999_999);
    assert_eq!(document.social_stats().len(), 5);
    assert_eq!(document.characters().len(), 10);
    assert_eq!(document.confidants().len(), 23);
    assert!(document.compendium_count() <= 232);

    let packed = document.to_bytes().expect("pack");
    let reloaded = SaveContainer::from_bytes(&packed, true).expect("reload packed");
    assert_eq!(reloaded.data, document.data);
    assert_eq!(reloaded.header.full_name(), document.header.full_name());
    assert!(reloaded.is_pc_payload());
}

#[test]
fn synthetic_payload_round_trips() {
    let mut container = SaveContainer {
        data: (0..DATA_SIZE).map(|i| (i % 251) as u8).collect(),
        ..SaveContainer::default()
    };
    container.data[0..4].copy_from_slice(&0x31u32.to_le_bytes());
    container.header.set_full_name("テスト 名前");
    container.header.playtime = 12345;
    let packed = container.pack(true, true, true).expect("pack");
    let reloaded = SaveContainer::from_bytes(&packed, true).expect("reload");
    assert_eq!(reloaded.data, container.data);
    assert_eq!(reloaded.header.full_name(), "テスト 名前");
    assert_eq!(reloaded.header.playtime, 12345);
    assert!(reloaded.is_pc_payload());
    assert!(reloaded.file_crc_ok && reloaded.data_crc_ok);
}

fn synthetic_document() -> SaveDocument {
    let mut container = SaveContainer {
        data: vec![0u8; DATA_SIZE],
        ..SaveContainer::default()
    };
    container.data[0..4].copy_from_slice(&0x31u32.to_le_bytes());
    container.header.desc =
        "3月20日(月) 朝 クリアデータ\nPLV 99 テスト\nPLAY TIME 87時間09分\n難易度 NORMAL\n　\n"
            .into();
    let header = container.header.clone();
    let data = container.data.clone();
    SaveDocument {
        path: PathBuf::from("DATA05/DATA.DAT"),
        container,
        data: data.clone(),
        header: header.clone(),
        original_data: data,
        original_header: header,
        dirty: false,
        crc_warning: String::new(),
    }
}

#[test]
fn date_changes_header_payload_and_description() {
    let mut document = synthetic_document();
    document.set_date(267, 4).expect("set date");
    assert_eq!(document.header.day, 267);
    assert_eq!(document.header.time, 4);
    assert_eq!(document.date(), (267, 4));
    assert_eq!(document.next_date(), (267, 4));
    assert!(document
        .header
        .desc
        .starts_with("12月24日(土) 放課後 クリアデータ"));
    assert!(document.header.desc.contains("\nPLV 99 テスト\n"));

    let packed = document.to_bytes().expect("pack");
    let reloaded = SaveContainer::from_bytes(&packed, true).expect("reload");
    assert_eq!(
        reloaded.data[0xA670..0xA673],
        [0x0B, 0x01, 0x04],
        "payload date bytes"
    );
}

#[test]
fn invalid_date_values_are_rejected() {
    let mut document = synthetic_document();
    assert!(document.set_date(365, 4).is_err());
    assert!(document.set_date(10, 7).is_err());
    assert!(!document.dirty);
}

#[test]
fn no_op_writes_do_not_dirty_the_document() {
    let mut document = synthetic_document();
    document.set_money(0);
    document.set_playtime_minutes(0);
    document.set_difficulty(2);
    assert!(!document.dirty);
    document.set_confidant(0, 0, 0).expect("no-op confidant");
    assert!(!document.dirty);
    document.set_money(1234);
    assert!(document.dirty);
    document.revert();
    assert!(!document.dirty);
}

#[test]
fn name_change_writes_the_payload_name() {
    let mut document = synthetic_document();
    document.set_name("ああ いい");
    assert_eq!(document.header.full_name(), "ああ いい");
    assert_eq!(
        &document.data[NAME_OFFSET..NAME_OFFSET + "ああ いい".len()],
        "ああ いい".as_bytes()
    );
}

#[test]
fn unchanged_persona_apply_does_not_dirty() {
    let mut document = synthetic_document();
    let base = 44 + 0x38;
    document.data[base..base + 2].copy_from_slice(&1u16.to_le_bytes());
    document.data[base + 2..base + 4].copy_from_slice(&100u16.to_le_bytes());
    document.data[base + 4] = 50;
    document.data[base + 0x1C..base + 0x21].copy_from_slice(&[10; 5]);
    let edit = PersonaEdit {
        id: 100,
        level: 50,
        xp: 0,
        trait_id: 0,
        stats: [10; 5],
        skills: [0; 8],
        clear: false,
    };
    document.apply_persona(0, 0, &edit).expect("persona apply");
    assert!(!document.dirty);
    let mut changed = edit.clone();
    changed.level = 51;
    document
        .apply_persona(0, 0, &changed)
        .expect("persona change");
    assert!(document.dirty);
}

#[test]
fn equipment_stacks_store_counts() {
    let mut document = synthetic_document();
    document.set_item_value("armor", 10, 42);
    document.set_item_value("accessory", 1, 7);
    assert_eq!(document.item_value("armor", 10), 42);
    assert_eq!(document.item_value("accessory", 1), 7);
    document.max_items("armor");
    assert_eq!(document.item_value("armor", 10), 99);
    document.clear_items("armor");
    assert_eq!(document.item_value("armor", 10), 0);
}

#[test]
fn calendar_and_playtime_helpers_are_exposed() {
    assert_eq!(calendar::parse_date("12/24"), Some(267));
    assert_eq!(calendar::format_date(353), "3月20日(月)");
    assert_eq!(input::parse_playtime_minutes("87:09"), Some(5229));
}
