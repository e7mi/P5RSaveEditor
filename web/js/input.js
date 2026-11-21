// Number and text normalisation shared by the editor fields.

// Converts full-width ASCII, full-width spaces and common separators to plain input.
export function normalizeText(text) {
  let out = '';
  for (const ch of text) {
    const code = ch.codePointAt(0);
    if (code === 0x3000) {
      out += ' ';
    } else if (code >= 0xff01 && code <= 0xff5e) {
      out += String.fromCodePoint(code - 0xfee0);
    } else if (code === 0xffe5) {
      out += ' ';
    } else {
      out += ch;
    }
  }
  return out;
}

// Parses an unsigned integer, allowing commas, spaces and full-width digits.
export function parseU32(text) {
  const cleaned = normalizeText(text).replace(/[\s,_¥￥]/g, '');
  if (!cleaned || !/^\d+$/.test(cleaned)) {
    return null;
  }
  const value = parseInt(cleaned, 10);
  return value <= 0xffffffff ? value : null;
}

export function parseU16(text) {
  const value = parseU32(text);
  return value !== null && value <= 0xffff ? value : null;
}

export function parseU8(text) {
  const value = parseU32(text);
  return value !== null && value <= 0xff ? value : null;
}

export function formatPlaytimeInput(minutes) {
  return `${Math.floor(minutes / 60)}:${String(minutes % 60).padStart(2, '0')}`;
}

// Parses `87:09`, `87時間9分`, `87h9m` or a bare minute count.
export function parsePlaytimeMinutes(text) {
  const normalized = normalizeText(text).trim();
  if (!normalized) {
    return null;
  }
  const cleaned = normalized
    .replace(/時間/g, ':')
    .replace(/[hH]/g, ':')
    .replace(/[分mM]/g, '');
  if (!cleaned.includes(':')) {
    return parseU32(cleaned);
  }
  const parts = cleaned.split(':');
  if (parts.length !== 2) {
    return null;
  }
  const hours = parseInt(parts[0].trim(), 10);
  const minutes = parseInt(parts[1].trim(), 10);
  if (Number.isNaN(hours) || Number.isNaN(minutes) || minutes >= 60) {
    return null;
  }
  const total = hours * 60 + minutes;
  return Number.isSafeInteger(total) ? total : null;
}

export function playtimeText(minutes) {
  return `${Math.floor(minutes / 60)}時間${String(minutes % 60).padStart(2, '0')}分`;
}

export function moneyText(value) {
  return '¥' + value.toLocaleString('en-US');
}

export function difficultyLabel(value) {
  switch (value) {
    case 0: return '安全';
    case 1: return '易しい';
    case 2: return '普通';
    case 3: return '難しい';
    case 4: return '容赦なし';
    default: return '不明';
  }
}
