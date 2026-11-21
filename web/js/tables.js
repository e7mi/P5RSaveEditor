// Embedded name tables and static game definitions.

import { GAMEDATA, CATEGORIES, CHARACTERS } from '../data.js';

export const categories = CATEGORIES;
export const characters = CHARACTERS;

const nameMap = (rows) =>
  new Map(
    rows
      .filter((row) => row.id !== 0)
      .map((row) => [row.id, row.name || row.name_en])
  );

const personaNames = nameMap(GAMEDATA.personas);
const skillNames = nameMap(GAMEDATA.skills);
const traitNames = nameMap(GAMEDATA.traits);

// Confidant list mirrors the Rust table merge: sort by save_id, drop the
// duplicate 信念 row (save_id 36) and record save_ids [33, 36] for it.
export const confidants = (() => {
  const rows = [...GAMEDATA.confidants].sort((a, b) => a.save_id - b.save_id);
  const defs = [];
  let seenFaith = false;
  for (const row of rows) {
    if (row.save_id === 36) {
      continue;
    }
    if (row.arcana === '信念') {
      if (seenFaith) {
        continue;
      }
      seenFaith = true;
      defs.push({
        save_ids: [33, 36],
        arcana: row.arcana,
        name: row.name || '芳澤すみれ',
        story_locked: row.story_locked || false,
      });
    } else {
      defs.push({
        save_ids: [row.save_id],
        arcana: row.arcana,
        name: row.name,
        story_locked: row.story_locked || false,
      });
    }
  }
  const order = (id) => (id >= 1 && id <= 21 ? id - 1 : id === 33 ? 21 : id === 35 ? 22 : 99);
  defs.sort((a, b) => order(a.save_ids[0]) - order(b.save_ids[0]));
  return defs;
})();

export const socialRows = GAMEDATA.social_stats;
export const personas = GAMEDATA.personas;
export const skills = GAMEDATA.skills;
export const traits = GAMEDATA.traits;
export const items = GAMEDATA.items;

export function personaName(id) {
  if (id === 0) {
    return '-';
  }
  return personaNames.get(id) ?? `ID ${id}`;
}

export function skillName(id) {
  if (id === 0) {
    return 'なし';
  }
  return skillNames.get(id) ?? `ID ${id}`;
}

export function traitName(id) {
  if (id === 0) {
    return 'なし';
  }
  return traitNames.get(id) ?? `ID ${id}`;
}

export function itemName(category, index) {
  const id = itemPrefix(category) + index;
  const rows = items[category];
  const row = rows && rows[index];
  if (!row) {
    return `ID ${id.toString(16).toUpperCase().padStart(4, '0')}`;
  }
  return row.name || row.name_en || `ID ${id.toString(16).toUpperCase().padStart(4, '0')}`;
}

export function category(key) {
  return categories.find((c) => c.key === key) ?? null;
}

export function itemPrefix(category) {
  switch (category) {
    case 'melee': return 0x1000;
    case 'ranged': return 0x7000;
    case 'armor': return 0x5000;
    case 'accessory': return 0x3000;
    case 'consumable': return 0x2000;
    case 'skillcard': return 0x4000;
    case 'material': return 0x8000;
    case 'outfit': return 0xa000;
    case 'keyitem': return 0x9000;
    default: return 0;
  }
}
