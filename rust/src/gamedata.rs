//! High-level editor model for the native PC (payload 0x31) save.

use std::path::{Path, PathBuf};

use crate::savefile::{
    encode_text, read_u16, read_u32, write_u16, write_u32, SaveContainer, SaveError, SaveHeader,
    SaveResult, DATA_SIZE,
};
use crate::tables::{tables, ConfidantDef};

pub const NAME_OFFSET: usize = 0x13840;
pub const NAME_LENGTH: usize = 0x40;
pub const MONEY_OFFSET: usize = 0x35C0;
pub const MONEY_MAX: u32 = 9_999_999;
pub const SOCIAL_OFFSET: usize = 0x139E0;
pub const SOCIAL_COUNT: usize = 5;
pub const SOCIAL_MAX_POINTS: u16 = 9999;
pub const CONFIDANT_OFFSET: usize = 0x136A0;
pub const CONFIDANT_STRIDE: usize = 16;
pub const CONFIDANT_SLOTS: usize = 23;
pub const CONFIDANT_ID_OFF: usize = 6;
pub const CONFIDANT_RANK_OFF: usize = 8;
pub const CONFIDANT_POINTS_OFF: usize = 10;
pub const COMPENDIUM_OFFSET: usize = 0x9973;
pub const COMPENDIUM_MIRROR: usize = 0x21E83;
pub const COMPENDIUM_BYTES: usize = 29;
pub const COMPENDIUM_MAX_ID: u16 = 232;
pub const MIRROR_DELTA: usize = 0x18510;
pub const DATE_OFFSET: usize = 0xA670;
pub const NEXT_DATE_OFFSET: usize = 0xA674;
pub const PERSONA_OFFSET: usize = 0x38;
pub const PERSONA_SLOTS: usize = 12;
pub const PERSONA_STRIDE: usize = 0x30;

#[derive(Clone, Debug)]
pub struct PersonaView {
    pub slot: usize,
    pub flags: u16,
    pub id: u16,
    pub name: String,
    pub level: u8,
    pub xp: u32,
    pub trait_id: u16,
    pub trait_name: String,
    pub stats: [u8; 5],
    pub skills: [u16; 8],
    pub empty: bool,
}

#[derive(Clone, Debug)]
pub struct CharacterView {
    pub index: usize,
    pub key: String,
    pub name: String,
    pub joker: bool,
    pub hp: u32,
    pub sp: u32,
    pub level: u32,
    pub xp: u32,
    pub persona0: PersonaView,
}

#[derive(Clone, Debug)]
pub struct SocialView {
    pub index: usize,
    pub key: String,
    pub name: String,
    pub points: u16,
    pub rank: u8,
    pub next: Option<u16>,
}

#[derive(Clone, Debug)]
pub struct ConfidantView {
    pub index: usize,
    pub def: ConfidantDef,
    pub slot: Option<usize>,
    pub save_id: u16,
    pub rank: u16,
    pub points: u16,
}

impl ConfidantView {
    pub fn started(&self) -> bool {
        self.slot.is_some() && self.save_id != 0
    }
}

#[derive(Clone, Debug)]
pub struct ItemCategoryView {
    pub key: String,
    pub label: String,
    pub count: usize,
    pub kind: String,
    pub read_only: bool,
}

#[derive(Clone, Debug)]
pub struct PersonaEdit {
    pub id: u16,
    pub level: u8,
    pub xp: u32,
    pub trait_id: u16,
    pub stats: [u8; 5],
    pub skills: [u16; 8],
    pub clear: bool,
}

pub struct SaveDocument {
    pub path: PathBuf,
    pub container: SaveContainer,
    pub data: Vec<u8>,
    pub header: SaveHeader,
    pub original_data: Vec<u8>,
    pub original_header: SaveHeader,
    pub dirty: bool,
    pub crc_warning: String,
}

impl SaveDocument {
    pub fn load(path: &Path) -> SaveResult<Self> {
        let raw = std::fs::read(path)?;
        let container = SaveContainer::from_bytes(&raw, false)?;
        if !container.is_pc_payload() {
            return Err(SaveError::Message(
                "このセーブは PC ネイティブ形式 (payload 0x31) ではありません。PS4 から変換したセーブには対応していません。".into(),
            ));
        }
        if container.data.len() < DATA_SIZE {
            return Err(SaveError::Message(format!(
                "ペイロードサイズが想定外です ({} bytes)。このエディタは {} bytes の Steam 版セーブを対象にしています。",
                container.data.len(),
                DATA_SIZE
            )));
        }
        let mut crc_warning = String::new();
        if !container.file_crc_ok {
            crc_warning = "外側 CRC が一致しません（保存時に再計算されます）".into();
        } else if !container.data_crc_ok {
            crc_warning = "データ CRC が一致しません（保存時に再計算されます）".into();
        }
        let data = container.data.clone();
        let header = container.header.clone();
        Ok(Self {
            path: path.to_path_buf(),
            container,
            data: data.clone(),
            header: header.clone(),
            original_data: data,
            original_header: header,
            dirty: false,
            crc_warning,
        })
    }

    pub fn slot_name(&self) -> String {
        self.path
            .parent()
            .and_then(|parent| parent.file_name())
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| "(未保存)".into())
    }

    pub fn is_clear_data(&self) -> bool {
        self.header.desc.contains("クリアデータ")
    }

    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    pub fn revert(&mut self) {
        self.data = self.original_data.clone();
        self.header = self.original_header.clone();
        self.container.data = self.data.clone();
        self.container.header = self.header.clone();
        self.dirty = false;
    }

    pub fn to_bytes(&mut self) -> SaveResult<Vec<u8>> {
        if self.data.len() != DATA_SIZE {
            return Err(SaveError::Message(format!(
                "ペイロード長が変更されています ({} != {})。破損を防ぐため保存を中止しました。",
                self.data.len(),
                DATA_SIZE
            )));
        }
        self.container.data = self.data.clone();
        self.container.header = self.header.clone();
        self.container.pack(true, true, true)
    }

    pub fn mark_saved(&mut self, _packed: &[u8]) {
        self.original_data = self.data.clone();
        self.original_header = self.header.clone();
        self.dirty = false;
        self.crc_warning.clear();
    }

    // Money / social / confidants
    pub fn money(&self) -> u32 {
        read_u32(&self.data, MONEY_OFFSET)
    }

    pub fn set_money(&mut self, value: u32) {
        let value = value.min(MONEY_MAX);
        if self.money() == value {
            return;
        }
        write_u32(&mut self.data, MONEY_OFFSET, value);
        self.mark_dirty();
    }

    pub fn social_stats(&self) -> Vec<SocialView> {
        tables()
            .social_rows()
            .iter()
            .enumerate()
            .map(|(index, row)| {
                let points = read_u16(&self.data, SOCIAL_OFFSET + index * 2);
                let rank = row
                    .thresholds
                    .iter()
                    .filter(|&&threshold| threshold <= points)
                    .count()
                    .clamp(1, 5) as u8;
                let next = if (rank as usize) < 5 {
                    row.thresholds.get(rank as usize).copied()
                } else {
                    None
                };
                SocialView {
                    index,
                    key: row.key.clone(),
                    name: row.name.clone(),
                    points,
                    rank,
                    next,
                }
            })
            .collect()
    }

    pub fn set_social_points(&mut self, index: usize, points: u16) {
        if index >= SOCIAL_COUNT {
            return;
        }
        let points = points.min(SOCIAL_MAX_POINTS);
        let offset = SOCIAL_OFFSET + index * 2;
        if read_u16(&self.data, offset) == points {
            return;
        }
        write_u16(&mut self.data, offset, points);
        self.mark_dirty();
    }

    pub fn max_social(&mut self) {
        for (index, row) in tables().social_rows().iter().enumerate() {
            if let Some(&max) = row.thresholds.last() {
                self.set_social_points(index, max);
            }
        }
    }

    fn confidant_slot_id(&self, slot: usize) -> u16 {
        read_u16(
            &self.data,
            CONFIDANT_OFFSET + slot * CONFIDANT_STRIDE + CONFIDANT_ID_OFF,
        )
    }

    pub fn confidants(&self) -> Vec<ConfidantView> {
        tables()
            .confidants
            .iter()
            .enumerate()
            .map(|(index, def)| {
                let mut slot = None;
                let mut save_id = 0;
                for candidate in 0..CONFIDANT_SLOTS {
                    let entry_id = self.confidant_slot_id(candidate);
                    if def.save_ids.contains(&entry_id) {
                        slot = Some(candidate);
                        save_id = entry_id;
                        break;
                    }
                }
                let (rank, points) = if let Some(slot) = slot {
                    let base = CONFIDANT_OFFSET + slot * CONFIDANT_STRIDE;
                    (
                        read_u16(&self.data, base + CONFIDANT_RANK_OFF),
                        read_u16(&self.data, base + CONFIDANT_POINTS_OFF),
                    )
                } else {
                    (0, 0)
                };
                ConfidantView {
                    index,
                    def: def.clone(),
                    slot,
                    save_id,
                    rank,
                    points,
                }
            })
            .collect()
    }

    fn ensure_confidant_slot(&mut self, def: &ConfidantDef) -> SaveResult<usize> {
        for index in 0..CONFIDANT_SLOTS {
            if def.save_ids.contains(&self.confidant_slot_id(index)) {
                return Ok(index);
            }
        }
        for index in 0..CONFIDANT_SLOTS {
            if self.confidant_slot_id(index) == 0 {
                let base = CONFIDANT_OFFSET + index * CONFIDANT_STRIDE;
                write_u16(&mut self.data, base + CONFIDANT_ID_OFF, def.save_ids[0]);
                write_u16(&mut self.data, base + CONFIDANT_RANK_OFF, 0);
                write_u16(&mut self.data, base + CONFIDANT_POINTS_OFF, 0);
                self.mark_dirty();
                return Ok(index);
            }
        }
        Err(SaveError::Message(
            "コープの空きスロットがありません。".into(),
        ))
    }

    pub fn set_confidant(&mut self, index: usize, rank: u16, points: u16) -> SaveResult<()> {
        let def = tables()
            .confidants
            .get(index)
            .cloned()
            .ok_or_else(|| SaveError::Message("コープ定義が見つかりません。".into()))?;
        let rank = rank.min(10);
        let points = points.min(999);
        if rank == 0 && points == 0 {
            if let Some(state) = self.confidants().get(index) {
                if !state.started() {
                    return Ok(());
                }
            }
        }
        let slot = self.ensure_confidant_slot(&def)?;
        let base = CONFIDANT_OFFSET + slot * CONFIDANT_STRIDE;
        if read_u16(&self.data, base + CONFIDANT_RANK_OFF) == rank
            && read_u16(&self.data, base + CONFIDANT_POINTS_OFF) == points
        {
            return Ok(());
        }
        write_u16(&mut self.data, base + CONFIDANT_RANK_OFF, rank);
        write_u16(&mut self.data, base + CONFIDANT_POINTS_OFF, points);
        self.mark_dirty();
        Ok(())
    }

    pub fn max_all_confidants(&mut self) -> SaveResult<()> {
        for index in 0..tables().confidants.len() {
            self.set_confidant(index, 10, 0)?;
        }
        Ok(())
    }

    // Characters / personas
    fn character_base(&self, index: usize) -> SaveResult<usize> {
        tables()
            .characters
            .get(index)
            .map(|character| character.base)
            .ok_or_else(|| SaveError::Message("キャラクター定義が見つかりません。".into()))
    }

    pub fn characters(&self) -> Vec<CharacterView> {
        tables()
            .characters
            .iter()
            .enumerate()
            .filter_map(|(index, definition)| {
                let base = definition.base;
                let persona0 = self.persona_view(index, 0).ok()?;
                let (level, xp) = if definition.joker {
                    (
                        read_u32(&self.data, base + 0xC),
                        read_u32(&self.data, base + 0x10),
                    )
                } else {
                    (persona0.level as u32, persona0.xp)
                };
                Some(CharacterView {
                    index,
                    key: definition.key.clone(),
                    name: definition.name.clone(),
                    joker: definition.joker,
                    hp: read_u32(&self.data, base),
                    sp: read_u32(&self.data, base + 4),
                    level,
                    xp,
                    persona0,
                })
            })
            .collect()
    }

    pub fn set_character_hp(&mut self, index: usize, value: u32) -> SaveResult<()> {
        let base = self.character_base(index)?;
        let value = value.min(9999);
        if read_u32(&self.data, base) == value {
            return Ok(());
        }
        write_u32(&mut self.data, base, value);
        self.mark_dirty();
        Ok(())
    }

    pub fn set_character_sp(&mut self, index: usize, value: u32) -> SaveResult<()> {
        let base = self.character_base(index)?;
        let value = value.min(9999);
        if read_u32(&self.data, base + 4) == value {
            return Ok(());
        }
        write_u32(&mut self.data, base + 4, value);
        self.mark_dirty();
        Ok(())
    }

    pub fn set_character_level(&mut self, index: usize, value: u32) -> SaveResult<()> {
        let joker = tables()
            .characters
            .get(index)
            .map(|character| character.joker)
            .ok_or_else(|| SaveError::Message("キャラクター定義が見つかりません。".into()))?;
        let level = value.clamp(1, 99);
        if joker {
            let base = self.character_base(index)?;
            if read_u32(&self.data, base + 0xC) == level {
                return Ok(());
            }
            write_u32(&mut self.data, base + 0xC, level);
            self.header.level = level as u8;
            let name = self.header.full_name();
            self.header
                .update_desc(Some(&name), Some(level as u8), None, None);
            self.mark_dirty();
        } else {
            self.set_persona_level(index, 0, level as u8)?;
        }
        Ok(())
    }

    pub fn set_character_xp(&mut self, index: usize, value: u32) -> SaveResult<()> {
        let joker = tables()
            .characters
            .get(index)
            .map(|character| character.joker)
            .ok_or_else(|| SaveError::Message("キャラクター定義が見つかりません。".into()))?;
        if joker {
            let base = self.character_base(index)?;
            if read_u32(&self.data, base + 0x10) == value {
                return Ok(());
            }
            write_u32(&mut self.data, base + 0x10, value);
            self.mark_dirty();
        } else {
            self.set_persona_xp(index, 0, value)?;
        }
        Ok(())
    }

    fn persona_base(&self, character_index: usize, slot: usize) -> SaveResult<usize> {
        if slot >= PERSONA_SLOTS {
            return Err(SaveError::Message("ペルソナスロットが不正です。".into()));
        }
        Ok(self.character_base(character_index)? + PERSONA_OFFSET + slot * PERSONA_STRIDE)
    }

    pub fn persona_view(&self, character_index: usize, slot: usize) -> SaveResult<PersonaView> {
        let base = self.persona_base(character_index, slot)?;
        let id = read_u16(&self.data, base + 2);
        let mut skills = [0u16; 8];
        for (i, skill) in skills.iter_mut().enumerate() {
            *skill = read_u16(&self.data, base + 0xC + i * 2);
        }
        let mut stats = [0u8; 5];
        stats.copy_from_slice(&self.data[base + 0x1C..base + 0x21]);
        let trait_id = read_u16(&self.data, base + 6);
        Ok(PersonaView {
            slot,
            flags: read_u16(&self.data, base),
            id,
            name: tables().persona_name(id),
            level: self.data[base + 4],
            xp: read_u32(&self.data, base + 8),
            trait_id,
            trait_name: tables().trait_name(trait_id),
            stats,
            skills,
            empty: id == 0,
        })
    }

    pub fn personas(&self, character_index: usize) -> Vec<PersonaView> {
        (0..PERSONA_SLOTS)
            .filter_map(|slot| self.persona_view(character_index, slot).ok())
            .collect()
    }

    pub fn set_persona_level(
        &mut self,
        character_index: usize,
        slot: usize,
        value: u8,
    ) -> SaveResult<()> {
        let base = self.persona_base(character_index, slot)?;
        let value = value.clamp(1, 99);
        if self.data[base + 4] == value {
            return Ok(());
        }
        self.data[base + 4] = value;
        self.mark_dirty();
        Ok(())
    }

    pub fn set_persona_xp(
        &mut self,
        character_index: usize,
        slot: usize,
        value: u32,
    ) -> SaveResult<()> {
        let base = self.persona_base(character_index, slot)?;
        if read_u32(&self.data, base + 8) == value {
            return Ok(());
        }
        write_u32(&mut self.data, base + 8, value);
        self.mark_dirty();
        Ok(())
    }

    pub fn apply_persona(
        &mut self,
        character_index: usize,
        slot: usize,
        edit: &PersonaEdit,
    ) -> SaveResult<()> {
        let base = self.persona_base(character_index, slot)?;
        let before = self.data[base..base + PERSONA_STRIDE].to_vec();
        if edit.clear {
            self.data[base..base + PERSONA_STRIDE].fill(0);
            if self.data[base..base + PERSONA_STRIDE] != before {
                self.mark_dirty();
            }
            return Ok(());
        }
        let old_flags = read_u16(&self.data, base);
        let old_id = read_u16(&self.data, base + 2);
        let was_empty = old_id == 0;
        if was_empty || old_id != edit.id {
            self.data[base..base + PERSONA_STRIDE].fill(0);
            write_u16(&mut self.data, base, 0x0001);
            write_u16(&mut self.data, base + 2, edit.id);
            self.data[base + 4] = edit.level.clamp(1, 99);
            write_u16(&mut self.data, base + 6, edit.trait_id);
            for (i, skill) in edit.skills.iter().enumerate() {
                write_u16(&mut self.data, base + 0xC + i * 2, *skill);
            }
            self.data[base + 0x1C..base + 0x21].copy_from_slice(&edit.stats);
            write_u32(&mut self.data, base + 8, edit.xp);
            if !was_empty {
                write_u16(&mut self.data, base, old_flags);
            }
        } else {
            self.data[base + 4] = edit.level.clamp(1, 99);
            write_u16(&mut self.data, base + 6, edit.trait_id);
            for (i, skill) in edit.skills.iter().enumerate() {
                write_u16(&mut self.data, base + 0xC + i * 2, *skill);
            }
            self.data[base + 0x1C..base + 0x21].copy_from_slice(&edit.stats);
            write_u32(&mut self.data, base + 8, edit.xp);
        }
        if self.data[base..base + PERSONA_STRIDE] != before {
            self.mark_dirty();
        }
        Ok(())
    }

    pub fn clear_persona(&mut self, character_index: usize, slot: usize) -> SaveResult<()> {
        let base = self.persona_base(character_index, slot)?;
        self.data[base..base + PERSONA_STRIDE].fill(0);
        self.mark_dirty();
        Ok(())
    }

    // Items
    pub fn categories(&self) -> Vec<ItemCategoryView> {
        tables()
            .categories
            .iter()
            .map(|category| ItemCategoryView {
                key: category.key.clone(),
                label: category.label.clone(),
                count: category.count,
                kind: category.kind.clone(),
                read_only: category.read_only,
            })
            .collect()
    }

    pub fn item_value(&self, category: &str, index: usize) -> u8 {
        tables()
            .category(category)
            .filter(|def| index < def.count)
            .map(|def| self.data[def.base + index])
            .unwrap_or(0)
    }

    pub fn set_item_value(&mut self, category: &str, index: usize, value: u8) {
        let Some(def) = tables().category(category) else {
            return;
        };
        if def.read_only || index >= def.count {
            return;
        }
        let value = if def.kind == "owned" {
            u8::from(value != 0)
        } else {
            value.min(99)
        };
        if self.data[def.base + index] == value {
            return;
        }
        self.data[def.base + index] = value;
        let mirror = def.base + MIRROR_DELTA + index;
        if mirror < self.data.len() {
            self.data[mirror] = value;
        }
        self.mark_dirty();
    }

    pub fn max_items(&mut self, category: &str) {
        let Some(def) = tables().category(category) else {
            return;
        };
        if def.read_only {
            return;
        }
        for index in 0..def.count {
            if def.kind == "owned" {
                if self.data[def.base + index] == 0 {
                    self.set_item_value(category, index, 1);
                }
            } else {
                self.set_item_value(category, index, 99);
            }
        }
    }

    pub fn clear_items(&mut self, category: &str) {
        let Some(def) = tables().category(category) else {
            return;
        };
        if def.read_only {
            return;
        }
        for index in 0..def.count {
            self.set_item_value(category, index, 0);
        }
    }

    // Compendium
    pub fn compendium_count(&self) -> usize {
        self.data[COMPENDIUM_OFFSET..COMPENDIUM_OFFSET + COMPENDIUM_BYTES]
            .iter()
            .map(|byte| byte.count_ones() as usize)
            .sum()
    }

    pub fn compendium_registered(&self, persona_id: u16) -> bool {
        if !(1..=COMPENDIUM_MAX_ID).contains(&persona_id) {
            return false;
        }
        let index = (persona_id - 1) as usize;
        let byte = index / 8;
        let bit = index % 8;
        self.data[COMPENDIUM_OFFSET + byte] & (1 << bit) != 0
    }

    pub fn set_compendium(&mut self, persona_id: u16, registered: bool) {
        if !(1..=COMPENDIUM_MAX_ID).contains(&persona_id) {
            return;
        }
        if self.compendium_registered(persona_id) == registered {
            return;
        }
        let index = (persona_id - 1) as usize;
        let byte = index / 8;
        let bit = 1u8 << (index % 8);
        for offset in [COMPENDIUM_OFFSET, COMPENDIUM_MIRROR] {
            if registered {
                self.data[offset + byte] |= bit;
            } else {
                self.data[offset + byte] &= !bit;
            }
        }
        self.mark_dirty();
    }

    pub fn all_compendium(&mut self) {
        for offset in [COMPENDIUM_OFFSET, COMPENDIUM_MIRROR] {
            self.data[offset..offset + COMPENDIUM_BYTES].fill(0xFF);
        }
        self.mark_dirty();
    }

    pub fn clear_compendium(&mut self) {
        for offset in [COMPENDIUM_OFFSET, COMPENDIUM_MIRROR] {
            self.data[offset..offset + COMPENDIUM_BYTES].fill(0);
        }
        self.mark_dirty();
    }

    // Header
    pub fn set_name(&mut self, full_name: &str) {
        let full_name = full_name.trim();
        if full_name.is_empty() {
            return;
        }
        let mut updated = self.header.clone();
        updated.set_full_name(full_name);
        let name = updated.full_name();
        if self.header.full_name() == name {
            return;
        }
        self.header = updated;
        self.header.update_desc(Some(&name), None, None, None);
        let encoded = encode_text(&name, NAME_LENGTH);
        self.data[NAME_OFFSET..NAME_OFFSET + NAME_LENGTH].copy_from_slice(&encoded);
        self.mark_dirty();
    }

    pub fn set_playtime_minutes(&mut self, minutes: u32) {
        let ticks = minutes.saturating_mul(60 * 30);
        if self.header.playtime == ticks {
            return;
        }
        self.header.playtime = ticks;
        self.header.update_desc(None, None, Some(ticks), None);
        self.mark_dirty();
    }

    pub fn set_difficulty(&mut self, difficulty: u8) {
        let difficulty = difficulty.min(4);
        if self.header.difficulty == difficulty {
            return;
        }
        self.header.difficulty = difficulty;
        self.header.update_desc(None, None, None, Some(difficulty));
        self.mark_dirty();
    }

    /// The date stored in the payload. Both the header and this pair are written by the game.
    pub fn date(&self) -> (u16, u8) {
        (
            read_u16(&self.data, DATE_OFFSET),
            self.data[DATE_OFFSET + 2],
        )
    }

    pub fn next_date(&self) -> (u16, u8) {
        (
            read_u16(&self.data, NEXT_DATE_OFFSET),
            self.data[NEXT_DATE_OFFSET + 2],
        )
    }

    /// Sets the current date and time of day in both the header and the payload.
    ///
    /// The next-date slot is only advanced when it would otherwise point before the new
    /// current date, so scheduled story dates stay untouched.
    pub fn set_date(&mut self, day: u16, time: u8) -> SaveResult<()> {
        if crate::calendar::day_to_date(day).is_none() {
            return Err(SaveError::Message("日付が範囲外です。".into()));
        }
        if !crate::calendar::is_valid_time(time) {
            return Err(SaveError::Message("時間帯が不正です。".into()));
        }
        if self.header.day == day && self.header.time == time && self.date() == (day, time) {
            return Ok(());
        }
        self.header.day = day;
        self.header.time = time;
        self.header.update_date_desc(day, time);
        write_u16(&mut self.data, DATE_OFFSET, day);
        self.data[DATE_OFFSET + 2] = time;
        let (next_day, next_time) = self.next_date();
        if next_day < day || (next_day == day && next_time < time) {
            write_u16(&mut self.data, NEXT_DATE_OFFSET, day);
            self.data[NEXT_DATE_OFFSET + 2] = time;
        }
        self.mark_dirty();
        Ok(())
    }

    /// True when the quick-info header and the payload disagree about the date.
    pub fn date_mismatch(&self) -> bool {
        let (day, time) = self.date();
        day != 0 && (day != self.header.day || time != self.header.time)
    }
}
