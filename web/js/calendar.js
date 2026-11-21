// P5R calendar helpers. Day 0 is April 1 of the game year, which was a Friday.

export const DAY_MIN = 0;
export const DAY_MAX = 364;

// Time-of-day values used by the save format, in display order.
export const TIMES = [
  [0, '朝'],
  [1, '昼'],
  [2, '午後'],
  [3, '日中'],
  [4, '放課後'],
  [5, '夜'],
  [6, '深夜'],
  [8, 'ダンジョン後'],
  [9, '深夜2'],
];

export function monthDays(month) {
  switch (month) {
    case 1: case 3: case 5: case 7: case 8: case 10: case 12: return 31;
    case 4: case 6: case 9: case 11: return 30;
    case 2: return 28;
    default: return null;
  }
}

export function dateToDay(month, day) {
  const limit = monthDays(month);
  if (limit === null || day === 0 || day > limit) {
    return null;
  }
  let total = 0;
  if (month >= 4) {
    for (let m = 4; m < month; m++) {
      total += monthDays(m);
    }
  } else {
    for (let m = 4; m <= 12; m++) {
      total += monthDays(m);
    }
    for (let m = 1; m < month; m++) {
      total += monthDays(m);
    }
  }
  total += day - 1;
  return total <= DAY_MAX ? total : null;
}

export function dayToDate(day) {
  if (day > DAY_MAX) {
    return null;
  }
  let remaining = day;
  for (const month of [4, 5, 6, 7, 8, 9, 10, 11, 12, 1, 2, 3]) {
    const length = monthDays(month);
    if (remaining < length) {
      return [month, remaining + 1];
    }
    remaining -= length;
  }
  return null;
}

const WEEKDAYS = ['金', '土', '日', '月', '火', '水', '木'];

export function weekday(day) {
  return WEEKDAYS[day % 7];
}

export function formatDate(day) {
  const date = dayToDate(day);
  return date ? `${date[0]}月${date[1]}日(${weekday(day)})` : `範囲外(${day})`;
}

export function formatDateShort(day) {
  const date = dayToDate(day);
  return date ? `${date[0]}/${date[1]}` : '?';
}

export function timeLabel(time) {
  const hit = TIMES.find(([value]) => value === time);
  return hit ? hit[1] : '不明';
}

// The shorter label written into the save-select description.
export function timeDescLabel(time) {
  switch (time) {
    case 0: return '朝';
    case 1: return '昼';
    case 2: return '午後';
    case 3: return '午後';
    case 4: return '放課後';
    case 5: return '夜';
    case 6: return '深夜';
    case 8: return '夜';
    case 9: return '深夜';
    default: return '不明';
  }
}

export function isValidTime(time) {
  return TIMES.some(([value]) => value === time);
}

import { normalizeText } from './input.js';

// Parses `12/24`, `12月24日`, `12-24` and `12 24`, normalising full-width input.
export function parseDate(text) {
  const normalized = normalizeText(text).trim();
  if (!normalized) {
    return null;
  }
  const replaced = normalized.replace(/月/g, '/').replace(/日/g, '').replace(/[-. ]/g, '/');
  const parts = replaced.split('/').filter((part) => part !== '');
  if (parts.length !== 2) {
    return null;
  }
  const month = parseInt(parts[0], 10);
  const day = parseInt(parts[1], 10);
  if (Number.isNaN(month) || Number.isNaN(day)) {
    return null;
  }
  return dateToDay(month, day);
}
