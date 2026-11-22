import { existsSync, readFileSync } from 'node:fs';
import { test } from 'node:test';
import assert from 'node:assert/strict';

import { SaveContainer, SaveHeader, SaveError, DATA_SIZE } from '../web/js/savefile.js';
import {
  SaveDocument,
  CONFIDANT_ID_OFF,
  CONFIDANT_OFFSET,
  CONFIDANT_POINTS_OFF,
  CONFIDANT_RANK_OFF,
  CONFIDANT_SLOTS,
  CONFIDANT_STRIDE,
  MIRROR_DELTA,
  NAME_OFFSET,
  NEXT_DATE_OFFSET,
} from '../web/js/gamedata.js';
import { category, confidants } from '../web/js/tables.js';
import { readU16, writeU16 } from '../web/js/util.js';
import { formatDate, parseDate } from '../web/js/calendar.js';
import { parsePlaytimeMinutes } from '../web/js/input.js';

const REAL_SAVE = process.env.P5R_TEST_SAVE ?? 'H:/0001/games/persona/testsave/DATA05/DATA.DAT';

test('real save reads and round trips', { skip: existsSync(REAL_SAVE) ? false : `real save not found: ${REAL_SAVE}` }, async () => {
  const raw = new Uint8Array(readFileSync(REAL_SAVE));
  const document = await SaveDocument.load(raw);
  assert.equal(document.data.length, DATA_SIZE);
  assert.equal(document.container.file_crc_ok, true);
  assert.equal(document.container.data_crc_ok, true);
  assert.ok(document.money() <= 9999999);
  assert.equal(document.socialStats().length, 5);
  assert.equal(document.characterViews().length, 10);
  assert.equal(document.confidantViews().length, 23);
  assert.ok(document.compendiumCount() <= 232);

  const packed = await document.toBytes();
  const reloaded = await SaveContainer.fromBytes(packed, true);
  assert.deepEqual([...reloaded.data], [...document.data]);
  assert.equal(reloaded.header.fullName(), document.header.fullName());
  assert.equal(reloaded.isPcPayload(), true);
});

const syntheticDocument = () => {
  const container = new SaveContainer({
    data: new Uint8Array(DATA_SIZE),
  });
  container.data[0] = 0x31;
  const header = new SaveHeader();
  header.desc = '3月20日(月) 朝 クリアデータ\nPLV 99 テスト\nPLAY TIME 87時間09分\n難易度 NORMAL\n　\n';
  return new SaveDocument({
    slotName: 'DATA05',
    container,
    data: container.data.slice(),
    header,
    originalData: container.data.slice(),
    originalHeader: new SaveHeader(JSON.parse(JSON.stringify(header))),
  });
};

test('synthetic payload round trips', async () => {
  const container = new SaveContainer({
    data: new Uint8Array(DATA_SIZE).map((_, i) => i % 251),
    header: new SaveHeader(),
  });
  container.data.set([0x31, 0, 0, 0]);
  container.header.setFullName('テスト 名前');
  container.header.playtime = 12345;
  const packed = await container.pack(true, true, true);
  const reloaded = await SaveContainer.fromBytes(packed, true);
  assert.deepEqual([...reloaded.data], [...container.data]);
  assert.equal(reloaded.header.fullName(), 'テスト 名前');
  assert.equal(reloaded.header.playtime, 12345);
  assert.equal(reloaded.isPcPayload(), true);
  assert.equal(reloaded.file_crc_ok, true);
  assert.equal(reloaded.data_crc_ok, true);
});

test('date changes header payload and description', async () => {
  const document = syntheticDocument();
  document.setDate(267, 4);
  assert.equal(document.header.day, 267);
  assert.equal(document.header.time, 4);
  assert.deepEqual(document.date(), [267, 4]);
  assert.deepEqual(document.nextDate(), [267, 4]);
  assert.ok(document.header.desc.startsWith('12月24日(土) 放課後 クリアデータ'));
  assert.ok(document.header.desc.includes('\nPLV 99 テスト\n'));

  const packed = await document.toBytes();
  const reloaded = await SaveContainer.fromBytes(packed, true);
  assert.deepEqual([...reloaded.data.slice(0xa670, 0xa673)], [0x0b, 0x01, 0x04]);
});

test('set_date preserves a future next_date', () => {
  const document = syntheticDocument();
  document.data[NEXT_DATE_OFFSET] = 0x61;
  document.data[NEXT_DATE_OFFSET + 1] = 0x01;
  document.data[NEXT_DATE_OFFSET + 2] = 5;
  document.setDate(267, 4);
  assert.deepEqual(document.date(), [267, 4]);
  assert.deepEqual(document.nextDate(), [353, 5]);
});

test('invalid date values are rejected', () => {
  const document = syntheticDocument();
  assert.throws(() => document.setDate(365, 4), SaveError);
  assert.throws(() => document.setDate(10, 7), SaveError);
  assert.equal(document.dirty, false);
});

test('no-op writes do not dirty the document', () => {
  const document = syntheticDocument();
  document.setMoney(0);
  document.setPlaytimeMinutes(0);
  document.setDifficulty(2);
  assert.equal(document.dirty, false);
  document.setConfidant(0, 0, 0);
  assert.equal(document.dirty, false);
  document.setMoney(1234);
  assert.equal(document.dirty, true);
  document.revert();
  assert.equal(document.dirty, false);
});

test('name change writes the payload name', () => {
  const document = syntheticDocument();
  document.setName('ああ いい');
  assert.equal(document.header.fullName(), 'ああ いい');
  const expected = new TextEncoder().encode('ああ いい');
  assert.deepEqual(
    [...document.data.subarray(NAME_OFFSET, NAME_OFFSET + expected.length)],
    [...expected]
  );
});

test('confidant allocation writes the first free slot', () => {
  const document = syntheticDocument();
  const saveId = confidants[0].save_ids[0];
  document.setConfidant(0, 5, 100);
  assert.equal(readU16(document.data, CONFIDANT_OFFSET + CONFIDANT_ID_OFF), saveId);
  assert.equal(readU16(document.data, CONFIDANT_OFFSET + CONFIDANT_RANK_OFF), 5);
  assert.equal(readU16(document.data, CONFIDANT_OFFSET + CONFIDANT_POINTS_OFF), 100);
  assert.equal(document.confidantViews()[0].started, true);
});

test('confidant allocation fails when full', () => {
  const document = syntheticDocument();
  for (let slot = 0; slot < CONFIDANT_SLOTS; slot++) {
    writeU16(
      document.data,
      CONFIDANT_OFFSET + slot * CONFIDANT_STRIDE + CONFIDANT_ID_OFF,
      1
    );
  }
  assert.throws(() => document.setConfidant(22, 5, 0), SaveError);
});

test('persona creation initializes an empty slot', () => {
  const document = syntheticDocument();
  const base = 44 + 0x38;
  document.applyPersona(0, 0, {
    id: 100,
    level: 50,
    xp: 1000,
    trait_id: 7,
    stats: [10, 10, 10, 10, 10],
    skills: [11, 22, 0, 0, 0, 0, 0, 0],
    clear: false,
  });
  assert.equal(document.dirty, true);
  assert.equal(readU16(document.data, base), 0x0001);
  assert.equal(readU16(document.data, base + 2), 100);
  assert.equal(readU16(document.data, base + 6), 7);
});

test('unchanged persona apply does not dirty', () => {
  const document = syntheticDocument();
  const base = 44 + 0x38;
  writeU16(document.data, base, 1);
  writeU16(document.data, base + 2, 100);
  document.data[base + 4] = 50;
  document.data.set([10, 10, 10, 10, 10], base + 0x1c);
  document.applyPersona(0, 0, {
    id: 100,
    level: 50,
    xp: 0,
    trait_id: 0,
    stats: [10, 10, 10, 10, 10],
    skills: [0, 0, 0, 0, 0, 0, 0, 0],
    clear: false,
  });
  assert.equal(document.dirty, false);
});

test('item write updates the mirror region', () => {
  const document = syntheticDocument();
  const base = category('armor').base;
  document.setItemValue('armor', 10, 42);
  assert.equal(document.data[base + MIRROR_DELTA + 10], 42);
});

test('equipment stacks store counts', () => {
  const document = syntheticDocument();
  document.setItemValue('armor', 10, 42);
  document.setItemValue('accessory', 1, 7);
  assert.equal(document.itemValue('armor', 10), 42);
  assert.equal(document.itemValue('accessory', 1), 7);
  document.maxItems('armor');
  assert.equal(document.itemValue('armor', 10), 99);
  document.clearItems('armor');
  assert.equal(document.itemValue('armor', 10), 0);
});

test('calendar and playtime helpers are exposed', () => {
  assert.equal(parseDate('12/24'), 267);
  assert.equal(formatDate(353), '3月20日(月)');
  assert.equal(parsePlaytimeMinutes('87:09'), 5229);
});
