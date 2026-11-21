// High-level editor model for the native PC (payload 0x31) save.

import { encodeText, readU16, readU32, writeU16, writeU32 } from './util.js';
import { DATA_SIZE, SaveContainer, SaveError, SaveHeader } from './savefile.js';
import { dayToDate, isValidTime } from './calendar.js';
import {
  categories,
  category,
  characters,
  confidants,
  personaName,
  skillName,
  socialRows,
  traitName,
} from './tables.js';

export const NAME_OFFSET = 0x13840;
export const NAME_LENGTH = 0x40;
export const MONEY_OFFSET = 0x35c0;
export const MONEY_MAX = 9999999;
export const SOCIAL_OFFSET = 0x139e0;
export const SOCIAL_COUNT = 5;
export const SOCIAL_MAX_POINTS = 9999;
export const CONFIDANT_OFFSET = 0x136a0;
export const CONFIDANT_STRIDE = 16;
export const CONFIDANT_SLOTS = 23;
export const CONFIDANT_ID_OFF = 6;
export const CONFIDANT_RANK_OFF = 8;
export const CONFIDANT_POINTS_OFF = 10;
export const COMPENDIUM_OFFSET = 0x9973;
export const COMPENDIUM_MIRROR = 0x21e83;
export const COMPENDIUM_BYTES = 29;
export const COMPENDIUM_MAX_ID = 232;
export const MIRROR_DELTA = 0x18510;
export const DATE_OFFSET = 0xa670;
export const NEXT_DATE_OFFSET = 0xa674;
export const PERSONA_OFFSET = 0x38;
export const PERSONA_SLOTS = 12;
export const PERSONA_STRIDE = 0x30;

export class SaveDocument {
  static async load(raw, slotName = 'DATA.DAT') {
    const container = await SaveContainer.fromBytes(raw, false);
    if (!container.isPcPayload()) {
      throw new SaveError(
        'このセーブは PC ネイティブ形式 (payload 0x31) ではありません。PS4 から変換したセーブには対応していません。'
      );
    }
    if (container.data.length < DATA_SIZE) {
      throw new SaveError(
        `ペイロードサイズが想定外です (${container.data.length} bytes)。このエディタは ${DATA_SIZE} bytes の Steam 版セーブを対象にしています。`
      );
    }
    let crcWarning = '';
    if (!container.file_crc_ok) {
      crcWarning = '外側 CRC が一致しません（保存時に再計算されます）';
    } else if (!container.data_crc_ok) {
      crcWarning = 'データ CRC が一致しません（保存時に再計算されます）';
    }
    const header = new SaveHeader(JSON.parse(JSON.stringify(container.header)));
    return new SaveDocument({
      slotName,
      container,
      data: container.data.slice(),
      header,
      originalData: container.data.slice(),
      originalHeader: new SaveHeader(JSON.parse(JSON.stringify(container.header))),
      crcWarning,
    });
  }

  constructor(fields) {
    Object.assign(this, { dirty: false }, fields);
  }

  isClearData() {
    return this.header.desc.includes('クリアデータ');
  }

  markDirty() {
    this.dirty = true;
  }

  revert() {
    this.data = this.originalData.slice();
    this.header = new SaveHeader(JSON.parse(JSON.stringify(this.originalHeader)));
    this.container.data = this.data.slice();
    this.container.header = new SaveHeader(JSON.parse(JSON.stringify(this.originalHeader)));
    this.dirty = false;
  }

  async toBytes() {
    if (this.data.length !== DATA_SIZE) {
      throw new SaveError(
        `ペイロード長が変更されています (${this.data.length} != ${DATA_SIZE})。破損を防ぐため保存を中止しました。`
      );
    }
    this.container.data = this.data.slice();
    this.container.header = new SaveHeader(JSON.parse(JSON.stringify(this.header)));
    return this.container.pack(true, true, true);
  }

  markSaved() {
    this.originalData = this.data.slice();
    this.originalHeader = new SaveHeader(JSON.parse(JSON.stringify(this.header)));
    this.dirty = false;
    this.crcWarning = '';
  }

  money() {
    return readU32(this.data, MONEY_OFFSET);
  }

  setMoney(value) {
    const clamped = Math.min(value, MONEY_MAX);
    if (this.money() === clamped) {
      return;
    }
    writeU32(this.data, MONEY_OFFSET, clamped);
    this.markDirty();
  }

  socialStats() {
    return socialRows.map((row, index) => {
      const points = readU16(this.data, SOCIAL_OFFSET + index * 2);
      const rank = Math.min(
        5,
        Math.max(1, row.thresholds.filter((threshold) => threshold <= points).length)
      );
      const next = rank < 5 ? row.thresholds[rank] : null;
      return { index, key: row.key, name: row.name, points, rank, next };
    });
  }

  setSocialPoints(index, points) {
    if (index >= SOCIAL_COUNT) {
      return;
    }
    const clamped = Math.min(points, SOCIAL_MAX_POINTS);
    const offset = SOCIAL_OFFSET + index * 2;
    if (readU16(this.data, offset) === clamped) {
      return;
    }
    writeU16(this.data, offset, clamped);
    this.markDirty();
  }

  maxSocial() {
    socialRows.forEach((row, index) => {
      const max = row.thresholds[row.thresholds.length - 1];
      if (max !== undefined) {
        this.setSocialPoints(index, max);
      }
    });
  }

  confidantSlotId(slot) {
    return readU16(this.data, CONFIDANT_OFFSET + slot * CONFIDANT_STRIDE + CONFIDANT_ID_OFF);
  }

  confidantViews() {
    return confidants.map((def, index) => {
      let slot = null;
      let saveId = 0;
      for (let candidate = 0; candidate < CONFIDANT_SLOTS; candidate++) {
        const entryId = this.confidantSlotId(candidate);
        if (def.save_ids.includes(entryId)) {
          slot = candidate;
          saveId = entryId;
          break;
        }
      }
      const base = slot === null ? 0 : CONFIDANT_OFFSET + slot * CONFIDANT_STRIDE;
      return {
        index,
        def,
        slot,
        save_id: saveId,
        rank: slot === null ? 0 : readU16(this.data, base + CONFIDANT_RANK_OFF),
        points: slot === null ? 0 : readU16(this.data, base + CONFIDANT_POINTS_OFF),
        started: slot !== null && saveId !== 0,
      };
    });
  }

  ensureConfidantSlot(def) {
    for (let index = 0; index < CONFIDANT_SLOTS; index++) {
      if (def.save_ids.includes(this.confidantSlotId(index))) {
        return index;
      }
    }
    for (let index = 0; index < CONFIDANT_SLOTS; index++) {
      if (this.confidantSlotId(index) === 0) {
        const base = CONFIDANT_OFFSET + index * CONFIDANT_STRIDE;
        writeU16(this.data, base + CONFIDANT_ID_OFF, def.save_ids[0]);
        writeU16(this.data, base + CONFIDANT_RANK_OFF, 0);
        writeU16(this.data, base + CONFIDANT_POINTS_OFF, 0);
        this.markDirty();
        return index;
      }
    }
    throw new SaveError('コープの空きスロットがありません。');
  }

  setConfidant(index, rank, points) {
    const def = confidants[index];
    if (!def) {
      throw new SaveError('コープ定義が見つかりません。');
    }
    const clampedRank = Math.min(rank, 10);
    const clampedPoints = Math.min(points, 999);
    if (clampedRank === 0 && clampedPoints === 0) {
      const state = this.confidantViews()[index];
      if (state && !state.started) {
        return;
      }
    }
    const slot = this.ensureConfidantSlot(def);
    const base = CONFIDANT_OFFSET + slot * CONFIDANT_STRIDE;
    if (
      readU16(this.data, base + CONFIDANT_RANK_OFF) === clampedRank &&
      readU16(this.data, base + CONFIDANT_POINTS_OFF) === clampedPoints
    ) {
      return;
    }
    writeU16(this.data, base + CONFIDANT_RANK_OFF, clampedRank);
    writeU16(this.data, base + CONFIDANT_POINTS_OFF, clampedPoints);
    this.markDirty();
  }

  maxAllConfidants() {
    for (let index = 0; index < confidants.length; index++) {
      this.setConfidant(index, 10, 0);
    }
  }

  characterBase(index) {
    const def = characters[index];
    if (!def) {
      throw new SaveError('キャラクター定義が見つかりません。');
    }
    return def.base;
  }

  characterViews() {
    return characters
      .map((definition, index) => {
        const base = definition.base;
        let persona0;
        try {
          persona0 = this.personaView(index, 0);
        } catch {
          return null;
        }
        const [level, xp] = definition.joker
          ? [readU32(this.data, base + 0xc), readU32(this.data, base + 0x10)]
          : [persona0.level, persona0.xp];
        return {
          index,
          key: definition.key,
          name: definition.name,
          joker: !!definition.joker,
          hp: readU32(this.data, base),
          sp: readU32(this.data, base + 4),
          level,
          xp,
          persona0,
        };
      })
      .filter((view) => view !== null);
  }

  setCharacterHp(index, value) {
    const base = this.characterBase(index);
    const clamped = Math.min(value, 9999);
    if (readU32(this.data, base) === clamped) {
      return;
    }
    writeU32(this.data, base, clamped);
    this.markDirty();
  }

  setCharacterSp(index, value) {
    const base = this.characterBase(index);
    const clamped = Math.min(value, 9999);
    if (readU32(this.data, base + 4) === clamped) {
      return;
    }
    writeU32(this.data, base + 4, clamped);
    this.markDirty();
  }

  setCharacterLevel(index, value) {
    const def = characters[index];
    if (!def) {
      throw new SaveError('キャラクター定義が見つかりません。');
    }
    const level = Math.min(99, Math.max(1, value));
    if (def.joker) {
      const base = this.characterBase(index);
      if (readU32(this.data, base + 0xc) === level) {
        return;
      }
      writeU32(this.data, base + 0xc, level);
      this.header.level = level;
      this.header.updateDesc(level, null, null);
      this.markDirty();
    } else {
      this.setPersonaLevel(index, 0, level);
    }
  }

  setCharacterXp(index, value) {
    const def = characters[index];
    if (!def) {
      throw new SaveError('キャラクター定義が見つかりません。');
    }
    if (def.joker) {
      const base = this.characterBase(index);
      if (readU32(this.data, base + 0x10) === value) {
        return;
      }
      writeU32(this.data, base + 0x10, value);
      this.markDirty();
    } else {
      this.setPersonaXp(index, 0, value);
    }
  }

  personaBase(characterIndex, slot) {
    if (slot >= PERSONA_SLOTS) {
      throw new SaveError('ペルソナスロットが不正です。');
    }
    return this.characterBase(characterIndex) + PERSONA_OFFSET + slot * PERSONA_STRIDE;
  }

  personaView(characterIndex, slot) {
    const base = this.personaBase(characterIndex, slot);
    const id = readU16(this.data, base + 2);
    const skills = [];
    for (let i = 0; i < 8; i++) {
      skills.push(readU16(this.data, base + 0xc + i * 2));
    }
    const stats = Array.from(this.data.subarray(base + 0x1c, base + 0x21));
    const traitId = readU16(this.data, base + 6);
    return {
      slot,
      flags: readU16(this.data, base),
      id,
      name: personaName(id),
      level: this.data[base + 4],
      xp: readU32(this.data, base + 8),
      trait_id: traitId,
      trait_name: traitName(traitId),
      stats,
      skills,
      empty: id === 0,
    };
  }

  personas(characterIndex) {
    const views = [];
    for (let slot = 0; slot < PERSONA_SLOTS; slot++) {
      try {
        views.push(this.personaView(characterIndex, slot));
      } catch {
        break;
      }
    }
    return views;
  }

  setPersonaLevel(characterIndex, slot, value) {
    const base = this.personaBase(characterIndex, slot);
    const level = Math.min(99, Math.max(1, value));
    if (this.data[base + 4] === level) {
      return;
    }
    this.data[base + 4] = level;
    this.markDirty();
  }

  setPersonaXp(characterIndex, slot, value) {
    const base = this.personaBase(characterIndex, slot);
    if (readU32(this.data, base + 8) === value) {
      return;
    }
    writeU32(this.data, base + 8, value);
    this.markDirty();
  }

  applyPersona(characterIndex, slot, edit) {
    const base = this.personaBase(characterIndex, slot);
    const before = this.data.slice(base, base + PERSONA_STRIDE);
    const changed = () =>
      !this.data.subarray(base, base + PERSONA_STRIDE).every((v, i) => v === before[i]);
    if (edit.clear) {
      this.data.fill(0, base, base + PERSONA_STRIDE);
      if (changed()) {
        this.markDirty();
      }
      return;
    }
    const oldFlags = readU16(this.data, base);
    const oldId = readU16(this.data, base + 2);
    const wasEmpty = oldId === 0;
    if (wasEmpty || oldId !== edit.id) {
      this.data.fill(0, base, base + PERSONA_STRIDE);
      writeU16(this.data, base, 0x0001);
      writeU16(this.data, base + 2, edit.id);
      this.data[base + 4] = Math.min(99, Math.max(1, edit.level));
      writeU16(this.data, base + 6, edit.trait_id);
      edit.skills.forEach((skill, i) => writeU16(this.data, base + 0xc + i * 2, skill));
      this.data.set(edit.stats, base + 0x1c);
      writeU32(this.data, base + 8, edit.xp);
      if (!wasEmpty) {
        writeU16(this.data, base, oldFlags);
      }
    } else {
      this.data[base + 4] = Math.min(99, Math.max(1, edit.level));
      writeU16(this.data, base + 6, edit.trait_id);
      edit.skills.forEach((skill, i) => writeU16(this.data, base + 0xc + i * 2, skill));
      this.data.set(edit.stats, base + 0x1c);
      writeU32(this.data, base + 8, edit.xp);
    }
    if (changed()) {
      this.markDirty();
    }
  }

  clearPersona(characterIndex, slot) {
    const base = this.personaBase(characterIndex, slot);
    this.data.fill(0, base, base + PERSONA_STRIDE);
    this.markDirty();
  }

  itemValue(categoryKey, index) {
    const def = category(categoryKey);
    if (!def || index >= def.count) {
      return 0;
    }
    return this.data[def.base + index];
  }

  setItemValue(categoryKey, index, value) {
    const def = category(categoryKey);
    if (!def || def.read_only || index >= def.count) {
      return;
    }
    const clamped = def.kind === 'owned' ? (value !== 0 ? 1 : 0) : Math.min(value, 99);
    if (this.data[def.base + index] === clamped) {
      return;
    }
    this.data[def.base + index] = clamped;
    const mirror = def.base + MIRROR_DELTA + index;
    if (mirror < this.data.length) {
      this.data[mirror] = clamped;
    }
    this.markDirty();
  }

  maxItems(categoryKey) {
    const def = category(categoryKey);
    if (!def || def.read_only) {
      return;
    }
    for (let index = 0; index < def.count; index++) {
      if (def.kind === 'owned') {
        if (this.data[def.base + index] === 0) {
          this.setItemValue(categoryKey, index, 1);
        }
      } else {
        this.setItemValue(categoryKey, index, 99);
      }
    }
  }

  clearItems(categoryKey) {
    const def = category(categoryKey);
    if (!def || def.read_only) {
      return;
    }
    for (let index = 0; index < def.count; index++) {
      this.setItemValue(categoryKey, index, 0);
    }
  }

  compendiumCount() {
    let total = 0;
    for (let i = 0; i < COMPENDIUM_BYTES; i++) {
      let byte = this.data[COMPENDIUM_OFFSET + i];
      while (byte) {
        total += byte & 1;
        byte >>>= 1;
      }
    }
    return total;
  }

  compendiumRegistered(personaId) {
    if (personaId < 1 || personaId > COMPENDIUM_MAX_ID) {
      return false;
    }
    const index = personaId - 1;
    const byte = Math.floor(index / 8);
    const bit = index % 8;
    return (this.data[COMPENDIUM_OFFSET + byte] & (1 << bit)) !== 0;
  }

  setCompendium(personaId, registered) {
    if (personaId < 1 || personaId > COMPENDIUM_MAX_ID) {
      return;
    }
    if (this.compendiumRegistered(personaId) === registered) {
      return;
    }
    const index = personaId - 1;
    const byte = Math.floor(index / 8);
    const bit = 1 << (index % 8);
    for (const offset of [COMPENDIUM_OFFSET, COMPENDIUM_MIRROR]) {
      if (registered) {
        this.data[offset + byte] |= bit;
      } else {
        this.data[offset + byte] &= ~bit;
      }
    }
    this.markDirty();
  }

  allCompendium() {
    for (const offset of [COMPENDIUM_OFFSET, COMPENDIUM_MIRROR]) {
      this.data.fill(0xff, offset, offset + COMPENDIUM_BYTES);
    }
    this.markDirty();
  }

  clearCompendium() {
    for (const offset of [COMPENDIUM_OFFSET, COMPENDIUM_MIRROR]) {
      this.data.fill(0, offset, offset + COMPENDIUM_BYTES);
    }
    this.markDirty();
  }

  setName(fullName) {
    const trimmed = fullName.trim();
    if (!trimmed) {
      return;
    }
    const updated = new SaveHeader(JSON.parse(JSON.stringify(this.header)));
    updated.setFullName(trimmed);
    const name = updated.fullName();
    if (this.header.fullName() === name) {
      return;
    }
    this.header = updated;
    this.header.updateDesc(null, null, null);
    const encoded = encodeText(name, NAME_LENGTH);
    this.data.set(encoded, NAME_OFFSET);
    this.markDirty();
  }

  setPlaytimeMinutes(minutes) {
    const ticks = Math.min(minutes * 60 * 30, 0xffffffff) >>> 0;
    if (this.header.playtime === ticks) {
      return;
    }
    this.header.playtime = ticks;
    this.header.updateDesc(null, ticks, null);
    this.markDirty();
  }

  setDifficulty(difficulty) {
    const clamped = Math.min(difficulty, 4);
    if (this.header.difficulty === clamped) {
      return;
    }
    this.header.difficulty = clamped;
    this.header.updateDesc(null, null, clamped);
    this.markDirty();
  }

  date() {
    return [readU16(this.data, DATE_OFFSET), this.data[DATE_OFFSET + 2]];
  }

  nextDate() {
    return [readU16(this.data, NEXT_DATE_OFFSET), this.data[NEXT_DATE_OFFSET + 2]];
  }

  setDate(day, time) {
    if (dayToDate(day) === null) {
      throw new SaveError('日付が範囲外です。');
    }
    if (!isValidTime(time)) {
      throw new SaveError('時間帯が不正です。');
    }
    const [nextDay, nextTime] = this.nextDate();
    if (
      this.header.day === day &&
      this.header.time === time &&
      this.date()[0] === day &&
      this.date()[1] === time
    ) {
      return;
    }
    this.header.day = day;
    this.header.time = time;
    this.header.updateDateDesc(day, time);
    writeU16(this.data, DATE_OFFSET, day);
    this.data[DATE_OFFSET + 2] = time;
    if (nextDay < day || (nextDay === day && nextTime < time)) {
      writeU16(this.data, NEXT_DATE_OFFSET, day);
      this.data[NEXT_DATE_OFFSET + 2] = time;
    }
    this.markDirty();
  }

  dateMismatch() {
    const [day, time] = this.date();
    return day !== 0 && (day !== this.header.day || time !== this.header.time);
  }

  itemCategories() {
    return categories.map((c) => ({ ...c }));
  }
}

export function skillLabel(id) {
  return skillName(id);
}
