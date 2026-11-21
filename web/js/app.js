// Browser UI for the P5R (Steam, payload 0x31) save editor.
// Single-page app: one Editor state object, render() rebuilds the page DOM.

import { toFullwidth } from './util.js';
import { SaveContainer } from './savefile.js';
import {
  SaveDocument,
  MONEY_MAX,
  NAME_LENGTH,
} from './gamedata.js';
import {
  TIMES,
  formatDate,
  formatDateShort,
  isValidTime,
  parseDate,
  timeLabel,
} from './calendar.js';
import {
  difficultyLabel,
  formatPlaytimeInput,
  moneyText,
  parsePlaytimeMinutes,
  parseU8,
  parseU16,
  parseU32,
  playtimeText,
} from './input.js';
import {
  categories,
  characters,
  itemName,
  itemPrefix,
  personaName,
  personas,
  skillName,
  skills,
  socialRows,
  traitName,
  traits,
} from './tables.js';

const APP_VERSION = '2.0.0';
const canFs = 'showDirectoryPicker' in window;
const canSavePicker = 'showSaveFilePicker' in window;

// Reserve / placeholder names hidden unless "予備スロットを表示" is on.
function isReserveName(name) {
  return [
    '', '未使用', '???', '？？？', '----------', '－－－－－－',
    'BLANK', 'ブランク', 'ブランク（使用不可）', 'BLANK (Unusable)',
    '使用不可', 'リザーブ', 'RESERVE', '予備', 'Unused',
  ].includes(name.trim());
}

// ---------- small DOM helpers ----------

function h(tag, opts = {}, ...children) {
  const el = document.createElement(tag);
  if (opts.cls) el.className = opts.cls;
  if (opts.text !== undefined) el.textContent = opts.text;
  if (opts.style) el.style.cssText = opts.style;
  if (opts.type) el.type = opts.type;
  if (opts.value !== undefined) el.value = opts.value;
  if (opts.title) el.title = opts.title;
  if (opts.attrs) for (const [k, v] of Object.entries(opts.attrs)) el.setAttribute(k, v);
  if (opts.dataKey) el.dataset.fkey = opts.dataKey;
  for (const child of children.flat(Infinity)) {
    if (child !== null && child !== undefined) el.append(child);
  }
  return el;
}

function btn(label, onClick, cls = '') {
  const b = h('button', { cls, text: label });
  b.addEventListener('click', onClick);
  return b;
}

function chip(label, selected, onClick) {
  const c = btn(label, onClick, `chip${selected ? ' sel' : ''}`);
  return c;
}

function tag(text, kind = '') {
  return h('span', { cls: `tag${kind ? ' ' + kind : ''}`, text });
}

function cbox(on) {
  return h('span', { cls: `cbox${on ? ' on' : ''}` });
}

function checkToggle(on, label, onClick) {
  const row = h('span', { cls: 'check-toggle' }, cbox(on), h('span', { text: label }));
  row.addEventListener('click', onClick);
  return row;
}

function card(...children) {
  return h('div', { cls: 'card' }, ...children);
}

function cardHeader(title, ...actions) {
  return h('h3', {}, h('span', { text: title }), h('span', { cls: 'spacer' }), ...actions);
}

// A text draft: v is the uncommitted string shown in the input.
function Field(v = '') {
  return { v };
}

// Text input bound to a draft. parse() validates the draft text and returns
// the value to commit or null when invalid; invalid text stays in the draft
// but never reaches the document (marked .invalid instead).
function fieldInput(draft, parse, commit, width, key) {
  const input = h('input', { type: 'text', cls: 'input' });
  if (width) input.style.width = typeof width === 'number' ? `${width}px` : width;
  if (key) input.dataset.fkey = key;
  const mark = () => input.classList.toggle('invalid', parse(draft.v) === null);
  input.value = draft.v;
  input.addEventListener('input', () => {
    draft.v = input.value;
    mark();
  });
  input.addEventListener('change', () => {
    const parsed = parse(draft.v);
    if (parsed === null) {
      mark();
      return;
    }
    commit(parsed);
    render();
  });
  mark();
  return input;
}

function searchBox(state, key) {
  const input = h('input', { cls: 'plain', type: 'text', dataKey: key });
  input.value = state.q;
  input.addEventListener('input', () => {
    state.q = input.value;
    if (state.page !== undefined) state.page = 0;
    render();
  });
  return h(
    'span',
    { cls: 'search-box' },
    h('span', { cls: 'lbl', text: '検索' }),
    input
  );
}

// ---------- global state ----------

const Editor = {
  doc: null, // SaveDocument
  fileHandle: null, // FileSystemFileHandle used for direct write-back
  originalRaw: null, // untouched bytes from load time (backup download)
  slots: new Map(), // name -> { name, raw, handle, valid, label, preview }
  page: 'basic',
  status: '',
  statusKind: 'info', // info | error | success
  modal: null, // { kind, ... }
  pending: null, // () => void
  selectedCharacter: 0,
  selectedCategory: 0,
  itemSearch: { q: '', page: 0 },
  compSearch: { q: '', page: 0 },
  confSearch: { q: '' },
  ownedOnly: false,
  showReserve: false,
  advanced: false,
  dateTime: 4,
  persona: null, // persona edit modal state
  picker: null, // picker state inside the persona modal
  drafts: {
    basic: { name: Field(), money: Field('0'), playtime: Field('0:00'), date: Field('4/1'), social: [] },
    confRank: [],
    confPoints: [],
    members: [],
    items: [],
    itemCategory: '',
  },
};

function setStatus(kind, message) {
  Editor.statusKind = kind;
  Editor.status = message;
}

// ---------- file loading ----------

function download(bytes, filename) {
  const blob = new Blob([bytes], { type: 'application/octet-stream' });
  const url = URL.createObjectURL(blob);
  const a = h('a', { attrs: { href: url, download: filename } });
  document.body.append(a);
  a.click();
  a.remove();
  setTimeout(() => URL.revokeObjectURL(url), 5000);
}

function slotLabel(doc) {
  return `${doc.header.fullName() || '---'}  |  ${formatDate(doc.header.day)}  |  Lv ${doc.header.level}`;
}

async function preview(raw, name) {
  try {
    const doc = await SaveDocument.load(raw, name);
    return { valid: true, label: slotLabel(doc), doc };
  } catch {
    return { valid: false, label: '読み込み失敗' };
  }
}

async function scanFolder() {
  let dir;
  try {
    dir = await window.showDirectoryPicker();
  } catch {
    return;
  }
  setStatus('info', 'savedata フォルダをスキャン中...');
  render();
  Editor.slots = new Map();
  let found = 0;
  for (let n = 1; n <= 16; n++) {
    const name = `DATA${String(n).padStart(2, '0')}`;
    let entry = { name, raw: null, handle: null, valid: false, label: '空' };
    try {
      const sub = await dir.getDirectoryHandle(name);
      const handle = await sub.getFileHandle('DATA.DAT');
      const file = await handle.getFile();
      const raw = new Uint8Array(await file.arrayBuffer());
      const p = await preview(raw, name);
      entry = { name, raw, handle, valid: p.valid, label: p.label };
      if (p.valid) found++;
    } catch {
      // missing folder/file keeps the "空" entry
    }
    Editor.slots.set(name, entry);
  }
  Editor.modal = { kind: 'slots' };
  setStatus(found ? 'info' : 'error', found ? `${found} 件のセーブを検出しました。` : '有効なセーブが見つかりませんでした。');
  render();
}

async function openFiles(fileList) {
  for (const file of fileList) {
    const rel = file.webkitRelativePath;
    let name = file.name || 'DATA.DAT';
    if (rel) {
      const parts = rel.split('/');
      if (parts.length >= 2) name = parts[parts.length - 2] || name;
    }
    const raw = new Uint8Array(await file.arrayBuffer());
    const go = () => loadSlot(name, raw, null);
    if (Editor.doc && Editor.doc.dirty) {
      requestConfirm('未保存の変更', '未保存の変更があります。破棄して別のファイルを読み込みますか？', go);
      return;
    }
    await go();
  }
}

async function loadSlot(name, raw, handle) {
  try {
    const doc = await SaveDocument.load(raw, name);
    Editor.doc = doc;
    Editor.originalRaw = raw.slice();
    Editor.fileHandle = handle;
    Editor.slots.set(name, {
      name,
      raw: raw.slice(),
      handle,
      valid: true,
      label: slotLabel(doc),
    });
    Editor.modal = null;
    Editor.pending = null;
    Editor.persona = null;
    Editor.picker = null;
    syncAllDrafts();
    if (Editor.page === 'settings') Editor.page = 'basic';
    if (doc.crcWarning) {
      setStatus('error', `${name} を読み込みました。${doc.crcWarning}。保存すると CRC を再計算します。`);
    } else {
      setStatus('success', `${name} を読み込みました。`);
    }
  } catch (error) {
    setStatus('error', `読み込み失敗: ${error.message || error}`);
  }
  render();
}

function requestLoadSlot(name) {
  const entry = Editor.slots.get(name);
  if (!entry || !entry.valid) return;
  if (Editor.doc && Editor.doc.slotName === name) {
    Editor.modal = null;
    render();
    return;
  }
  const go = async () => {
    await loadSlot(entry.name, entry.raw.slice(), entry.handle);
  };
  if (Editor.doc && Editor.doc.dirty) {
    requestConfirm('未保存の変更', '未保存の変更があります。破棄して別のスロットを読み込みますか？', go);
  } else {
    go();
  }
}

// ---------- draft sync ----------

function syncAllDrafts() {
  syncBasicDrafts();
  syncConfidantDrafts();
  syncMemberDrafts();
  syncItemDrafts();
}

function syncBasicDrafts() {
  const doc = Editor.doc;
  if (!doc) return;
  const b = Editor.drafts.basic;
  b.name.v = doc.header.fullName();
  b.money.v = String(doc.money());
  b.playtime.v = formatPlaytimeInput(doc.header.playtimeMinutes());
  b.date.v = formatDateShort(doc.header.day);
  const payloadTime = doc.date()[1];
  // Keep the raw payload value when both reads are invalid so applyAll does
  // not silently rewrite an unknown time on save.
  Editor.dateTime = isValidTime(payloadTime) ? payloadTime : isValidTime(doc.header.time) ? doc.header.time : payloadTime;
  const stats = doc.socialStats();
  b.social = [];
  for (let i = 0; i < Math.max(5, stats.length); i++) b.social.push(Field('0'));
  stats.forEach((stat, i) => {
    b.social[i].v = String(stat.points);
  });
}

function syncConfidantDrafts() {
  const doc = Editor.doc;
  if (!doc) return;
  const states = doc.confidantViews();
  Editor.drafts.confRank = states.map((s) => Field(String(s.rank)));
  Editor.drafts.confPoints = states.map((s) => Field(String(s.points)));
}

function syncMemberDrafts() {
  const doc = Editor.doc;
  if (!doc) return;
  Editor.drafts.members = doc.characterViews().map((c) => ({
    hp: Field(String(c.hp)),
    sp: Field(String(c.sp)),
    level: Field(String(c.level)),
    xp: Field(String(c.xp)),
  }));
}

function currentCategory() {
  return categories[Math.min(Editor.selectedCategory, categories.length - 1)];
}

function syncItemDrafts() {
  const doc = Editor.doc;
  const cat = currentCategory();
  if (!doc || !cat) {
    Editor.drafts.items = [];
    Editor.drafts.itemCategory = '';
    return;
  }
  Editor.drafts.itemCategory = cat.key;
  Editor.drafts.items = [];
  for (let i = 0; i < cat.count; i++) {
    Editor.drafts.items.push(Field(String(doc.itemValue(cat.key, i))));
  }
}

// ---------- field parsers (same rules as desktop) ----------

const moneyValue = () => {
  const v = parseU32(Editor.drafts.basic.money.v);
  return v !== null && v <= MONEY_MAX ? v : null;
};
const playtimeValue = () => {
  const v = parsePlaytimeMinutes(Editor.drafts.basic.playtime.v);
  return v !== null && v * 60 * 30 <= 0xffffffff ? v : null;
};
const dateValue = () => parseDate(Editor.drafts.basic.date.v);
const socialValue = (i) => {
  const f = Editor.drafts.basic.social[i];
  const v = f ? parseU16(f.v) : null;
  return v !== null && v <= 9999 ? v : null;
};
const confRankValue = (i) => {
  const f = Editor.drafts.confRank[i];
  const v = f ? parseU16(f.v) : null;
  return v !== null && v <= 10 ? v : null;
};
const confPointsValue = (i) => {
  const f = Editor.drafts.confPoints[i];
  const v = f ? parseU16(f.v) : null;
  return v !== null && v <= 999 ? v : null;
};
const memberHpValue = (i) => {
  const f = Editor.drafts.members[i];
  const v = f ? parseU32(f.hp.v) : null;
  return v !== null && v <= 9999 ? v : null;
};
const memberSpValue = (i) => {
  const f = Editor.drafts.members[i];
  const v = f ? parseU32(f.sp.v) : null;
  return v !== null && v <= 9999 ? v : null;
};
const memberLevelValue = (i) => {
  const f = Editor.drafts.members[i];
  const v = f ? parseU32(f.level.v) : null;
  return v !== null && v >= 1 && v <= 99 ? v : null;
};
const memberXpValue = (i) => {
  const f = Editor.drafts.members[i];
  const v = f ? parseU32(f.xp.v) : null;
  return v !== null && v <= 2000000000 ? v : null;
};
const itemValueInput = (i) => {
  const f = Editor.drafts.items[i];
  const v = f ? parseU8(f.v) : null;
  return v !== null && v <= 99 ? v : null;
};

// ---------- save-time validation / apply (mirrors app.rs) ----------

function validateAll() {
  const doc = Editor.doc;
  const b = Editor.drafts.basic;
  if (b.name.v.trim() === '') return '名前を入力してください。';
  if (new TextEncoder().encode(toFullwidth(b.name.v.trim())).length > NAME_LENGTH)
    return `名前は ${NAME_LENGTH} バイト以内で入力してください。`;
  if (moneyValue() === null) return '所持金は 0〜9,999,999 の数値で入力してください。';
  if (playtimeValue() === null) return 'プレイ時間は「87:09」または分単位の数値で入力してください。';
  if (!doc.isClearData()) {
    if (dateValue() === null) return '日付は「12/24」の形式で入力してください。';
    if (!isValidTime(Editor.dateTime)) return '時間帯が不正です。';
  }
  for (let i = 0; i < b.social.length; i++) {
    if (socialValue(i) === null) return `${socialRows[i]?.name ?? 'ソーシャル'} のポイントが不正です。`;
  }
  for (let i = 0; i < Math.min(Editor.drafts.confRank.length, Editor.drafts.confPoints.length); i++) {
    if (confRankValue(i) === null) return 'コープのランクは 0〜10 で入力してください。';
    if (confPointsValue(i) === null) return 'コープのポイントは 0〜999 で入力してください。';
  }
  for (let i = 0; i < Editor.drafts.members.length; i++) {
    if (memberHpValue(i) === null || memberSpValue(i) === null) return 'HP / SP は 0〜9999 で入力してください。';
    if (memberLevelValue(i) === null) return 'レベルは 1〜99 で入力してください。';
    if (memberXpValue(i) === null) return 'EXP が不正です。';
  }
  const cat = currentCategory();
  if (cat && !cat.read_only && cat.kind !== 'owned') {
    for (let i = 0; i < cat.count; i++) {
      if (itemValueInput(i) === null) return `「${cat.label}」の個数は 0〜99 で入力してください。`;
    }
  }
  return null;
}

function applyAll() {
  const doc = Editor.doc;
  const b = Editor.drafts.basic;
  const name = b.name.v.trim();
  if (doc.header.fullName() !== name) doc.setName(name);
  const money = moneyValue();
  if (money !== null && doc.money() !== money) doc.setMoney(money);
  const playtime = playtimeValue();
  if (playtime !== null && doc.header.playtimeMinutes() !== playtime) doc.setPlaytimeMinutes(playtime);
  const day = dateValue();
  if (!doc.isClearData() && day !== null) {
    if (doc.header.day !== day || doc.header.time !== Editor.dateTime || doc.date()[0] !== day || doc.date()[1] !== Editor.dateTime) {
      doc.setDate(day, Editor.dateTime);
    }
  }
  doc.socialStats().forEach((stat, i) => {
    const v = socialValue(i);
    if (v !== null && stat.points !== v) doc.setSocialPoints(i, v);
  });
  for (let i = 0; i < Math.min(Editor.drafts.confRank.length, Editor.drafts.confPoints.length); i++) {
    const rank = confRankValue(i);
    const points = confPointsValue(i);
    if (rank === null || points === null) continue;
    const state = doc.confidantViews()[i];
    if (state && (state.rank !== rank || state.points !== points)) {
      doc.setConfidant(i, rank, points);
    }
  }
  const views = doc.characterViews();
  for (let i = 0; i < Editor.drafts.members.length; i++) {
    const view = views[i];
    if (!view) continue;
    const hp = memberHpValue(i);
    const sp = memberSpValue(i);
    const level = memberLevelValue(i);
    const xp = memberXpValue(i);
    if (hp !== null && view.hp !== hp) doc.setCharacterHp(i, hp);
    if (sp !== null && view.sp !== sp) doc.setCharacterSp(i, sp);
    if (level !== null && view.level !== level) doc.setCharacterLevel(i, level);
    if (xp !== null && view.xp !== xp) doc.setCharacterXp(i, xp);
  }
  const cat = currentCategory();
  if (cat && !cat.read_only && cat.kind !== 'owned') {
    for (let i = 0; i < cat.count; i++) {
      const v = itemValueInput(i);
      if (v !== null && doc.itemValue(cat.key, i) !== v) doc.setItemValue(cat.key, i, v);
    }
  }
}

// ---------- confirm / message plumbing ----------

function requestConfirm(title, message, action, cancelAction) {
  Editor.pending = action;
  Editor.modal = { kind: 'confirm', title, message, cancelAction };
  render();
}

function showMessage(title, message) {
  Editor.pending = null;
  Editor.modal = { kind: 'message', title, message };
  render();
}

function closeModal() {
  if (Editor.modal && Editor.modal.kind === 'picker') {
    Editor.picker = null;
    Editor.modal = { kind: 'persona' };
  } else {
    Editor.modal = null;
    Editor.pending = null;
    Editor.persona = null;
    Editor.picker = null;
  }
  render();
}

// ---------- actions ----------

async function saveDocument() {
  const doc = Editor.doc;
  if (!doc) {
    showMessage('保存', 'セーブが読み込まれていません。');
    return;
  }
  if (Editor.persona) {
    setStatus('error', 'ペルソナ編集を「適用」または「キャンセル」してから保存してください。');
    render();
    return;
  }
  const error = validateAll();
  if (error) {
    setStatus('error', error);
    render();
    return;
  }
  try {
    applyAll();
  } catch (e) {
    setStatus('error', e.message || String(e));
    render();
    return;
  }
  if (!doc.dirty) {
    setStatus('info', '保存する変更はありません。');
    render();
    return;
  }
  let packed;
  try {
    packed = await doc.toBytes();
  } catch (e) {
    showMessage('保存失敗', e.message || String(e));
    return;
  }
  // Re-verify the produced container before offering it for write-back.
  try {
    const verified = await SaveContainer.fromBytes(packed, true);
    if (!verified.isPcPayload() || verified.data.length !== doc.data.length) {
      showMessage('保存失敗', '検証後のペイロードが一致しません。');
      return;
    }
  } catch (e) {
    showMessage('保存失敗', `生成したセーブの検証に失敗しました。\n${e.message || e}`);
    return;
  }
  const slot = doc.slotName;
  const finish = () => {
    doc.markSaved();
    const entry = Editor.slots.get(slot);
    if (entry) {
      entry.raw = packed.slice();
      entry.label = slotLabel(doc);
      entry.handle = Editor.fileHandle;
    }
    syncAllDrafts();
    setStatus('success', `${slot} を保存しました。`);
    render();
  };
  if (Editor.fileHandle) {
    try {
      const writable = await Editor.fileHandle.createWritable();
      await writable.write(packed);
      await writable.close();
      finish();
      return;
    } catch (e) {
      if (e && e.name === 'AbortError') return;
      // fall through to the picker / download path
    }
  }
  const fallbackDownload = () => {
    download(packed, 'DATA.DAT');
    setStatus('info', 'DATA.DAT をダウンロードしました。savedata フォルダに上書きしてください。');
    render();
  };
  if (canSavePicker) {
    requestConfirm(
      '書き込み先を選択',
      '元のファイルへの書き込みハンドルがありません。名前を付けて保存しますか？「キャンセル」でダウンロードします。',
      async () => {
        try {
          const handle = await window.showSaveFilePicker({ suggestedName: 'DATA.DAT' });
          const writable = await handle.createWritable();
          await writable.write(packed);
          await writable.close();
          Editor.fileHandle = handle;
          finish();
        } catch (e) {
          if (e && e.name === 'AbortError') return;
          fallbackDownload();
        }
      },
      fallbackDownload
    );
  } else {
    fallbackDownload();
  }
}

function downloadOriginal() {
  const doc = Editor.doc;
  if (!doc || !Editor.originalRaw) {
    showMessage('バックアップ', 'セーブが読み込まれていません。');
    return;
  }
  const name = /^DATA\d+$/.test(doc.slotName) ? `${doc.slotName}-original.DAT` : 'DATA-original.DAT';
  download(Editor.originalRaw.slice(), name);
  setStatus('info', 'オリジナルのバックアップをダウンロードしました。');
  render();
}

// ---------- page: 基本 ----------

function basicPage() {
  const doc = Editor.doc;
  const page = h('div', { cls: 'page' });
  page.append(pageHeader('基本', 'セーブ情報・所持金・ソーシャルを編集します。'));

  if (doc.isClearData()) {
    page.append(h('div', { cls: 'warn-strip', text: 'クリアデータです。通常の進行データとは内容が異なります。' }));
  }
  if (doc.crcWarning) {
    page.append(h('div', { cls: 'warn-strip', text: `${doc.crcWarning}。保存すると CRC を再計算します。` }));
  }
  if (!doc.isClearData() && doc.dateMismatch()) {
    page.append(
      h('div', {
        cls: 'warn-strip',
        text: `ヘッダーの日付 (${formatDate(doc.header.day)}) とセーブ本体の日付 (${formatDate(doc.date()[0])}) が一致していません。日付を保存し直すと同期されます。`,
      })
    );
  }

  const b = Editor.drafts.basic;
  // info card
  const info = card(cardHeader('セーブ情報'));
  info.append(
    h('div', { cls: 'row' },
      h('span', { cls: 'field-label', text: '名前' }),
      fieldInput(b.name, (v) => (v.trim() ? v : null), (v) => doc.setName(v), 200)),
    h('div', { cls: 'row' },
      h('span', { cls: 'field-label', text: 'プレイ時間' }),
      fieldInput(b.playtime, () => playtimeValue(), (v) => doc.setPlaytimeMinutes(v), 92)),
    h('div', { cls: 'row' },
      h('span', { cls: 'field-label', text: '' }),
      h('span', {
        cls: 'hint',
        text: playtimeValue() !== null ? `= ${playtimeText(playtimeValue())}` : '例: 87:09 / 5229 (分)',
      })),
    h('div', { cls: 'row' },
      h('span', { cls: 'field-label', text: '難易度' }),
      ...[0, 1, 2, 3, 4].map((v) => chip(difficultyLabel(v), doc.header.difficulty === v, () => { doc.setDifficulty(v); render(); }))),
    h('div', { cls: 'row' },
      h('span', { cls: 'field-label', text: '日付' }),
      h('span', { text: formatDate(doc.header.day) }),
      h('span', { cls: 'hint', text: timeLabel(doc.header.time) })),
    h('div', { cls: 'row' },
      h('span', { cls: 'field-label', text: '主人公Lv' }),
      h('span', { text: String(doc.header.level) }))
  );

  // money card
  const money = card(cardHeader('所持金'));
  money.append(
    h('div', { cls: 'row' },
      h('span', { cls: 'field-label', text: '金額' }),
      fieldInput(b.money, () => moneyValue(), (v) => doc.setMoney(v), 150),
      btn('MAX', () =>
        requestConfirm('所持金 MAX', '所持金を 9,999,999 にします。実行しますか？', () => {
          doc.setMoney(MONEY_MAX);
          b.money.v = String(MONEY_MAX);
          setStatus('info', '所持金を MAX にしました（未保存）。');
          render();
        }), 'primary')),
    h('div', { cls: 'row' },
      h('span', { cls: 'field-label', text: '' }),
      h('span', {
        cls: 'hint',
        text: moneyValue() !== null ? `= ${moneyText(moneyValue())}` : '0〜9,999,999',
      }),
      h('span', { cls: 'spacer' }),
      h('span', { cls: 'hint', text: `現在 ${moneyText(doc.money())}` })),
    h('div', { cls: 'small', text: '変更は「保存」を押すまでファイルに書き込まれません。' })
  );

  page.append(h('div', { cls: 'two-col' }, info, money));

  // social card
  const stats = doc.socialStats();
  const social = card(cardHeader('ソーシャル'));
  if (Editor.advanced) {
    social.firstChild.append(btn('全MAX', () =>
      requestConfirm('ソーシャル MAX', '5 項目を MAX にします。実行しますか？', () => {
        doc.maxSocial();
        doc.socialStats().forEach((stat, i) => {
          if (b.social[i]) b.social[i].v = String(stat.points);
        });
        setStatus('info', 'ソーシャルステータスを MAX にしました（未保存）。');
        render();
      }), 'danger'));
  }
  const socialGrid = h('div', { cls: 'grid2' });
  stats.forEach((stat, i) => {
    const line = h('div', { cls: 'item-row' },
      h('div', { style: 'width:92px; flex:none; display:flex; flex-direction:column; gap:1px;' },
        h('span', { text: stat.name }),
        h('span', { cls: 'hint', text: `ランク ${stat.rank}` })),
      h('span', { cls: 'stars', text: '★'.repeat(stat.rank) }),
      fieldInput(b.social[i], () => socialValue(i), (v) => doc.setSocialPoints(i, v), 70),
      h('span', { cls: 'hint', style: 'width:56px; flex:none;', text: stat.next !== null ? `次 ${stat.next}` : '上限' }));
    if (Editor.advanced) {
      line.append(btn('MAX', () => {
        const max = socialRows[i]?.thresholds[socialRows[i].thresholds.length - 1];
        if (max !== undefined) {
          b.social[i].v = String(max);
          doc.setSocialPoints(i, max);
        }
        render();
      }, 'link'));
    }
    socialGrid.append(line);
  });
  social.append(socialGrid, h('div', { cls: 'small', text: 'ポイントはゲーム内のランク計算に使われます。' }));
  page.append(social);

  // date card (advanced only, hidden for clear data)
  if (Editor.advanced && !doc.isClearData()) {
    const date = card(cardHeader('日付と時間帯'));
    date.append(
      h('div', { cls: 'row' },
        h('span', { cls: 'field-label', text: '日付' }),
        fieldInput(b.date, () => dateValue(), (v) => commitDate(v), 92),
        h('span', {
          cls: 'hint',
          text: dateValue() !== null ? `${formatDate(dateValue())} ${timeLabel(Editor.dateTime)}` : '例: 12/24',
        }),
        h('span', { cls: 'spacer' }),
        h('span', { cls: 'small', text: `保存値 ${formatDate(doc.date()[0])}` })),
      h('div', { cls: 'row' },
        ...TIMES.map(([value, label]) =>
          chip(label, Editor.dateTime === value, () => {
            Editor.dateTime = value;
            const day = dateValue();
            if (day !== null) commitDate(day);
            render();
          }))),
      h('div', { cls: 'danger-strip', text: '日付の変更は危険です。ストーリー進行・イベント予定・コープの解放時期と矛盾する可能性があります。必ずバックアップを取ってください。' })
    );
    page.append(date);
  } else {
    page.append(h('div', {
      cls: 'small',
      style: 'padding:4px 8px;',
      text: doc.isClearData() ? 'クリアデータの日付は編集できません。' : '日付の変更は高度なモードで表示されます。',
    }));
  }
  return page;
}

function commitDate(day) {
  const doc = Editor.doc;
  try {
    doc.setDate(day, Editor.dateTime);
  } catch (e) {
    setStatus('error', e.message || String(e));
  }
}

// ---------- page: コープ ----------

function confidantsPage() {
  const doc = Editor.doc;
  const page = h('div', { cls: 'page' });
  const header = pageHeader('コープ', 'ランクとポイントを個別に編集します。');
  if (Editor.advanced) {
    header.append(btn('全MAX', () =>
      requestConfirm('コープ MAX', '23 件をすべてランク 10 にします。実行しますか？', () => {
        try {
          doc.maxAllConfidants();
          syncConfidantDrafts();
          setStatus('info', 'コープをすべて MAX にしました（未保存）。');
        } catch (e) {
          setStatus('error', e.message || String(e));
        }
        render();
      }), 'danger'));
  }
  page.append(header);

  const query = Editor.confSearch.q.trim().toLowerCase();
  const states = doc.confidantViews().filter(
    (s) => !query || s.def.name.toLowerCase().includes(query) || s.def.arcana.toLowerCase().includes(query)
  );

  page.append(
    h('div', { cls: 'row' },
      searchBox(Editor.confSearch, 'conf-search'),
      h('span', { cls: 'spacer' }),
      h('span', { cls: 'small', text: `${states.length} / 23 件` }))
  );

  const grid = h('div', { cls: 'grid2' });
  for (const state of states) {
    grid.append(confidantCard(doc, state));
  }
  page.append(card(states.length ? grid : h('div', { cls: 'hint', text: '該当するコープがありません。' })));
  page.append(h('div', {
    cls: 'small',
    text: 'ランクを下げても通過済みイベントは戻りません。ストーリー固定のコープは表示のみ有効な場合があります。',
  }));
  return page;
}

function confidantCard(doc, state) {
  const i = state.index;
  const meta = h('div', { cls: 'row', style: 'font-size:12px;' },
    h('span', { cls: 'hint', text: state.def.arcana }));
  if (state.def.story_locked) meta.append(tag('ストーリー', 'warn'));
  if (!state.started) meta.append(tag('未開始'));
  else if (state.rank >= 10) meta.append(tag('MAX', 'ok'));
  meta.append(
    h('span', { cls: 'spacer' }),
    h('span', { cls: 'hint', text: 'ランク' }),
    fieldInput(Editor.drafts.confRank[i], () => confRankValue(i), (rank) => {
      const points = doc.confidantViews()[i]?.points ?? 0;
      try {
        doc.setConfidant(i, rank, points);
      } catch (e) {
        setStatus('error', e.message || String(e));
      }
    }, 52),
    h('span', { cls: 'hint', text: 'ポイント' }),
    fieldInput(Editor.drafts.confPoints[i], () => confPointsValue(i), (points) => {
      const rank = doc.confidantViews()[i]?.rank ?? 0;
      try {
        doc.setConfidant(i, rank, points);
      } catch (e) {
        setStatus('error', e.message || String(e));
      }
    }, 60)
  );
  return h('div', { cls: 'co-card' },
    h('div', { cls: 'row' }, h('span', { cls: 'name', text: state.def.name })),
    meta);
}

// ---------- page: メンバー ----------

function membersPage() {
  const doc = Editor.doc;
  const page = h('div', { cls: 'page' });
  page.append(pageHeader('メンバー', 'HP・SP・レベル・EXP とペルソナを 1 画面で編集します。'));

  const views = doc.characterViews();
  if (!views.length) {
    page.append(card(h('div', { cls: 'hint', text: 'メンバー情報を読み取れませんでした。' })));
    return page;
  }
  const index = Math.min(Editor.selectedCharacter, views.length - 1);
  const joker = !!characters[index]?.joker;

  const list = card();
  list.classList.add('member-list');
  for (const c of views) {
    const row = h('div', { cls: `mrow${c.index === index ? ' active' : ''}` },
      h('div', { cls: 'mname', text: c.name }),
      h('div', { cls: 'msub', text: `Lv ${c.level}  HP ${c.hp}  SP ${c.sp}` }));
    row.addEventListener('click', () => {
      Editor.selectedCharacter = c.index;
      render();
    });
    list.append(row);
  }

  page.append(h('div', { cls: 'member-layout' }, list, memberDetail(doc, views[index], joker)));
  return page;
}

function memberDetail(doc, c, joker) {
  const i = c.index;
  const d = Editor.drafts.members[i] || { hp: Field('0'), sp: Field('0'), level: Field('1'), xp: Field('0') };
  const wrap = h('div', { style: 'display:flex; flex-direction:column; gap:12px; min-width:0;' });

  const stats = card(
    h('h3', {},
      h('span', { text: c.name }),
      h('span', { cls: 'spacer' }),
      tag(joker ? '主人公' : 'メンバー', joker ? 'accent' : '')));
  const commitMember = () => {
    const hp = memberHpValue(i);
    const sp = memberSpValue(i);
    const level = memberLevelValue(i);
    const xp = memberXpValue(i);
    try {
      if (hp !== null) doc.setCharacterHp(i, hp);
      if (sp !== null) doc.setCharacterSp(i, sp);
      if (level !== null) doc.setCharacterLevel(i, level);
      if (xp !== null) doc.setCharacterXp(i, xp);
    } catch (e) {
      setStatus('error', e.message || String(e));
    }
  };
  stats.append(
    h('div', { cls: 'row' },
      labelledField('HP', d.hp, () => memberHpValue(i), commitMember, 78),
      labelledField('SP', d.sp, () => memberSpValue(i), commitMember, 78),
      labelledField('レベル', d.level, () => memberLevelValue(i), commitMember, 64),
      labelledField('EXP', d.xp, () => memberXpValue(i), commitMember, 96)),
    h('div', {
      cls: 'small',
      text: joker
        ? '主人公のレベルと EXP はキャラクター側の値です。ペルソナのレベルとは別です。'
        : 'メンバーのレベルと EXP は専用ペルソナの値です。',
    }));
  wrap.append(stats);

  const personas = doc.personas(i);
  const slots = joker ? 12 : 1;
  const table = card(cardHeader('ペルソナ', h('span', { cls: 'sub', text: joker ? 'スロット 01 が装備中' : '専用ペルソナ' })));
  const t = h('div', { cls: 'ptable' });
  t.append(h('div', { cls: 'prow head' },
    h('span', { cls: 'c-slot', text: '枠' }),
    h('span', { cls: 'c-name', text: 'ペルソナ' }),
    h('span', { cls: 'c-lv', text: 'Lv' }),
    h('span', { cls: 'c-stats', text: '力/魔/耐/速/運' }),
    h('span', { cls: 'c-skills', text: 'スキル' }),
    h('span', { cls: 'c-act', text: '' })));
  for (let slot = 0; slot < slots; slot++) {
    const p = personas[slot];
    if (!p) continue;
    t.append(personaRow(i, p, joker));
  }
  table.append(t);
  wrap.append(table);
  return wrap;
}

function labelledField(label, draft, check, commit, width) {
  return h('div', { cls: 'row', style: 'gap:5px;' },
    h('span', { cls: 'hint', text: label }),
    fieldInput(draft, check, commit, width));
}

function personaRow(character, p, joker) {
  const equipped = joker && p.slot === 0 && !p.empty;
  const skillList = p.skills.filter((id) => id !== 0).map((id) => skillName(id));
  const skillsText = skillList.length ? skillList.slice(0, 4).join(' / ') : 'なし';
  const nameCol = h('div', { style: 'display:flex; flex-direction:column; gap:1px;' },
    h('span', { text: p.empty ? '空きスロット' : p.name, style: p.empty ? 'color:var(--text2)' : '' }),
    h('div', { cls: 'row', style: 'gap:5px;' },
      h('span', { cls: 'hint', text: `ID ${p.id}` }),
      equipped ? tag('装備中', 'accent') : joker && p.slot === 0 ? tag('先頭') : null));
  const row = h('div', { cls: 'prow' },
    h('span', { cls: 'c-slot', text: String(p.slot + 1).padStart(2, '0') }),
    h('span', { cls: 'c-name' }, nameCol),
    h('span', { cls: 'c-lv', text: p.empty ? '-' : String(p.level) }),
    h('span', { cls: 'c-stats', text: p.empty ? '-' : p.stats.slice(0, 5).join('/') }),
    h('span', { cls: 'c-skills', text: skillsText, title: skillList.join(' / ') }),
    h('span', { cls: 'c-act' }, btn(p.empty ? '作成' : '編集', () => openPersona(character, p.slot), 'link')));
  return row;
}

// ---------- persona edit modal + picker ----------

function openPersona(character, slot) {
  const doc = Editor.doc;
  if (!doc) return;
  const def = characters[character];
  let view;
  try {
    view = doc.personaView(character, slot);
  } catch {
    return;
  }
  const draft = {
    id: view.id,
    level: Math.max(1, view.level),
    xp: view.xp,
    trait_id: view.trait_id,
    stats: view.empty ? [5, 5, 5, 5, 5] : view.stats.slice(0, 5),
    skills: view.skills.slice(0, 8),
    clear: false,
  };
  Editor.selectedCharacter = character;
  Editor.persona = {
    character,
    slot,
    draft,
    idLocked: !def.joker,
    canClear: !!def.joker && slot !== 0 && !view.empty,
    level: Field(String(draft.level)),
    xp: Field(String(draft.xp)),
    stats: draft.stats.map((v) => Field(String(v))),
  };
  Editor.modal = { kind: 'persona' };
  render();
}

function personaFieldValues() {
  const p = Editor.persona;
  if (!p) return null;
  const level = parseU8(p.level.v);
  if (level === null || level < 1 || level > 99) return null;
  const xp = parseU32(p.xp.v);
  if (xp === null || xp > 2000000000) return null;
  const stats = [];
  for (const f of p.stats) {
    const v = parseU8(f.v);
    if (v === null || v > 99) return null;
    stats.push(v);
  }
  return { level, xp, stats };
}

function commitPersonaDraft() {
  const values = personaFieldValues();
  if (!values) return;
  Editor.persona.draft.level = values.level;
  Editor.persona.draft.xp = values.xp;
  Editor.persona.draft.stats = values.stats;
}

function applyPersona() {
  const p = Editor.persona;
  if (!p) return;
  if (!personaFieldValues()) {
    setStatus('error', 'ペルソナの数値が不正です。');
    render();
    return;
  }
  commitPersonaDraft();
  const doc = Editor.doc;
  if (p.draft.clear) {
    if (p.canClear) {
      doc.clearPersona(p.character, p.slot);
      setStatus('info', 'ペルソナスロットを空にしました（未保存）。');
    }
  } else if (p.draft.id === 0) {
    setStatus('error', 'ペルソナを選択してください。');
    render();
    return;
  } else {
    doc.applyPersona(p.character, p.slot, p.draft);
    setStatus('info', 'ペルソナを更新しました（未保存）。');
  }
  Editor.persona = null;
  Editor.modal = null;
  syncMemberDrafts();
  render();
}

function openPicker(kind, target) {
  Editor.picker = { kind, target, page: 0, q: '' };
  Editor.modal = { kind: 'picker' };
  render();
}

function pickerRows() {
  const picker = Editor.picker;
  if (!picker) return [];
  const query = picker.q.trim().toLowerCase();
  const matches = (id, name) =>
    !query || name.toLowerCase().includes(query) || String(id).includes(query);
  const filterRows = (rows, allowZero) => {
    const out = [];
    if (allowZero && matches(0, 'なし')) out.push([0, 'なし']);
    for (const row of rows) {
      if (row.id === 0) continue;
      const name = row.name || row.name_en || '';
      if (isReserveName(name)) continue;
      if (!matches(row.id, name)) continue;
      out.push([row.id, name]);
    }
    return out;
  };
  if (picker.kind === 'persona') return filterRows(personas, false);
  if (picker.kind === 'skill') return filterRows(skills, true);
  return filterRows(traits, true);
}

function pickerSelect(id) {
  const p = Editor.persona;
  const target = Editor.picker?.target;
  if (p && target) {
    if (target.type === 'personaId') {
      p.draft.id = id;
      p.draft.clear = false;
    } else if (target.type === 'trait') {
      p.draft.trait_id = id;
    } else if (target.type === 'skill') {
      if (target.slot < 8) p.draft.skills[target.slot] = id;
    }
  }
  Editor.picker = null;
  Editor.modal = { kind: 'persona' };
  render();
}

// ---------- page: 所持品 ----------

function itemIndices(doc) {
  const cat = currentCategory();
  if (!cat) return [];
  const query = Editor.itemSearch.q.trim().toLowerCase();
  const out = [];
  for (let i = 0; i < cat.count; i++) {
    const name = itemName(cat.key, i);
    if (!Editor.showReserve && isReserveName(name)) continue;
    if (query && !name.toLowerCase().includes(query)) continue;
    if (Editor.ownedOnly && doc.itemValue(cat.key, i) === 0) continue;
    out.push(i);
  }
  return out;
}

function itemsPage() {
  const doc = Editor.doc;
  const cat = currentCategory();
  const page = h('div', { cls: 'page' });
  const header = pageHeader('所持品', '装備・消耗品・スキルカードなどを編集します。キーアイテムは閲覧のみです。');
  if (Editor.advanced && !cat.read_only) {
    header.append(
      btn('一括所持 / 最大', () =>
        requestConfirm('一括所持 / 最大', `「${cat.label}」の全 ${cat.count} 件を所持/最大にします。実行しますか？`, () => {
          doc.maxItems(cat.key);
          syncItemDrafts();
          setStatus('info', 'カテゴリをすべて所持/最大にしました（未保存）。');
          render();
        }), 'danger'),
      btn('一括解除', () =>
        requestConfirm('一括解除', `「${cat.label}」の全 ${cat.count} 件を 0 にします。実行しますか？`, () => {
          doc.clearItems(cat.key);
          syncItemDrafts();
          setStatus('info', 'カテゴリをすべて外しました（未保存）。');
          render();
        }), 'danger'));
  }
  page.append(header);

  page.append(h('div', { cls: 'row' },
    ...categories.map((c, idx) =>
      chip(c.label, idx === Math.min(Editor.selectedCategory, categories.length - 1), () => {
        Editor.selectedCategory = idx;
        Editor.itemSearch.page = 0;
        syncItemDrafts();
        render();
      }))));

  const indices = itemIndices(doc);
  page.append(h('div', { cls: 'row' },
    searchBox(Editor.itemSearch, 'item-search'),
    checkToggle(Editor.ownedOnly, '所持のみ', () => {
      Editor.ownedOnly = !Editor.ownedOnly;
      Editor.itemSearch.page = 0;
      render();
    }),
    h('span', { cls: 'spacer' }),
    h('span', { cls: 'small', text: `${indices.length} / ${cat.count} 件` })));

  const pageSize = 40;
  const pages = Math.max(1, Math.ceil(indices.length / pageSize));
  const current = Math.min(Editor.itemSearch.page, pages - 1);
  const start = current * pageSize;
  const end = Math.min(start + pageSize, indices.length);

  const list = card();
  const grid = h('div', { cls: 'grid2' });
  for (let k = start; k < end; k++) {
    grid.append(itemRow(doc, cat, indices[k]));
  }
  list.append(indices.length ? grid : h('div', { cls: 'hint', text: '該当するアイテムがありません。' }));
  page.append(list);

  page.append(h('div', { cls: 'pager' },
    btn('前へ', () => { Editor.itemSearch.page = Math.max(0, current - 1); render(); }),
    h('span', { cls: 'small', text: `${current + 1} / ${pages}` }),
    btn('次へ', () => { Editor.itemSearch.page = Math.min(pages - 1, current + 1); render(); }),
    h('span', { cls: 'spacer' }),
    h('span', {
      cls: 'small',
      text: `表示 ${indices.length ? start + 1 : 0}〜${end} / ${indices.length} 件`,
    })));
  return page;
}

function itemRow(doc, cat, i) {
  const id = itemPrefix(cat.key) + i;
  const name = itemName(cat.key, i);
  const value = doc.itemValue(cat.key, i);
  const row = h('div', { cls: 'item-row' },
    h('span', { cls: 'iid', text: id.toString(16).toUpperCase().padStart(4, '0') }),
    h('span', { cls: 'iname', text: name }));
  if (cat.read_only) {
    row.append(h('span', { cls: 'hint', text: String(value) }), tag('閲覧のみ'));
  } else if (cat.kind === 'owned') {
    const owned = value > 0;
    const toggle = h('span', { cls: `own-toggle${owned ? ' owned' : ''}` },
      cbox(owned),
      h('span', { text: owned ? '所持' : '未所持' }));
    toggle.addEventListener('click', () => {
      doc.setItemValue(cat.key, i, owned ? 0 : 1);
      const f = Editor.drafts.items[i];
      if (f) f.v = String(owned ? 0 : 1);
      render();
    });
    row.append(toggle);
  } else {
    row.append(fieldInput(Editor.drafts.items[i], () => itemValueInput(i), (v) => doc.setItemValue(cat.key, i, v), 64));
  }
  return row;
}

// ---------- page: 図鑑 ----------

function compendiumPage() {
  const doc = Editor.doc;
  const page = h('div', { cls: 'page' });
  const header = pageHeader('図鑑', 'ペルソナ図鑑の登録ビットを編集します。');
  if (Editor.advanced) {
    header.append(
      btn('全登録', () =>
        requestConfirm('図鑑 全登録', '232 件すべての登録ビットを有効にします。実行しますか？', () => {
          doc.allCompendium();
          setStatus('info', '図鑑ビットを全登録にしました（未保存）。');
          render();
        }), 'danger'),
      btn('全解除', () =>
        requestConfirm('図鑑 全解除', '232 件すべての登録ビットを解除します。実行しますか？', () => {
          doc.clearCompendium();
          setStatus('info', '図鑑ビットを全解除しました（未保存）。');
          render();
        }), 'danger'));
  }
  page.append(header);

  const query = Editor.compSearch.q.trim().toLowerCase();
  const ids = [];
  for (let id = 1; id <= 232; id++) {
    if (!query || personaName(id).toLowerCase().includes(query)) ids.push(id);
  }

  page.append(h('div', { cls: 'row' },
    searchBox(Editor.compSearch, 'comp-search'),
    h('span', { cls: 'spacer' }),
    h('span', { cls: 'small', text: `登録 ${doc.compendiumCount()} / 232` })));

  const pageSize = 64;
  const pages = Math.max(1, Math.ceil(ids.length / pageSize));
  const current = Math.min(Editor.compSearch.page, pages - 1);
  const start = current * pageSize;
  const end = Math.min(start + pageSize, ids.length);

  const grid = card();
  for (let rowStart = start; rowStart < end; rowStart += 4) {
    const line = h('div', { cls: 'row', style: 'gap:3px; flex-wrap:nowrap;' });
    for (let k = 0; k < 4 && rowStart + k < end; k++) {
      const id = ids[rowStart + k];
      const registered = doc.compendiumRegistered(id);
      const cell = h('span', { cls: `comp-cell${registered ? '' : ' off'}`, style: 'flex:1; min-width:0;' },
        cbox(registered),
        h('span', { cls: 'pid', text: String(id).padStart(3, '0') }),
        h('span', { cls: 'pname', text: personaName(id), title: personaName(id) }));
      cell.addEventListener('click', () => {
        doc.setCompendium(id, !doc.compendiumRegistered(id));
        render();
      });
      line.append(cell);
    }
    grid.append(line);
  }
  if (!ids.length) grid.append(h('div', { cls: 'hint', text: '該当するペルソナがありません。' }));
  page.append(grid);

  page.append(h('div', { cls: 'pager' },
    btn('前へ', () => { Editor.compSearch.page = Math.max(0, current - 1); render(); }),
    h('span', { cls: 'small', text: `${current + 1} / ${pages}` }),
    btn('次へ', () => { Editor.compSearch.page = Math.min(pages - 1, current + 1); render(); }),
    h('span', { cls: 'spacer' }),
    h('span', { cls: 'small', text: 'クリックで登録を切り替えます。' })));
  return page;
}

// ---------- page: バックアップ ----------

function backupsPage() {
  const doc = Editor.doc;
  const page = h('div', { cls: 'page' });
  page.append(pageHeader('バックアップ', '編集前のオリジナルファイルをダウンロードで退避します。'));
  const c = card(cardHeader('オリジナルの退避'));
  c.append(
    h('div', {
      cls: 'small',
      text: 'ブラウザ版ではバックアップをアプリ内に保存しません。読み込んだ時点の DATA.DAT（編集前の状態）をそのままダウンロードします。保存する前に必ずダウンロードしてください。',
    }));
  if (doc) {
    c.append(h('div', { cls: 'row' },
      btn('オリジナルをダウンロード', downloadOriginal, 'primary'),
      h('span', { cls: 'hint', text: `${doc.slotName} の読み込み時点のファイル` })));
  } else {
    c.append(h('div', { cls: 'hint', text: 'セーブが読み込まれていません。' }));
  }
  page.append(c);
  return page;
}

// ---------- page: 設定 ----------

function settingsPage() {
  const page = h('div', { cls: 'page' });
  page.append(pageHeader('設定', '表示と動作の設定を変更します。'));

  const api = card(cardHeader('ファイルアクセス'));
  api.append(
    h('div', { cls: 'row' },
      h('span', { text: 'File System Access API' }),
      tag(canFs ? '利用可能' : '利用不可', canFs ? 'ok' : 'warn')),
    h('div', {
      cls: 'small',
      text: canFs
        ? 'フォルダスキャンと直接書き込みが使えます。savedata フォルダ (DATA01〜DATA16) を選択してください。'
        : 'このブラウザではフォルダスキャンと直接書き込みが使えません。保存はダウンロードになります。',
    }),
    canFs ? h('div', { cls: 'row' }, btn('savedata フォルダをスキャン', scanFolder)) : null
  );
  page.append(api);

  const options = card(cardHeader('動作'));
  options.append(
    settingRow(Editor.showReserve, '予備スロットを表示', '未使用・予備のアイテム行も一覧に表示します。', () => {
      Editor.showReserve = !Editor.showReserve;
      render();
    }),
    settingRow(Editor.advanced, '高度なモード', '日付変更や一括操作など、危険な操作を表示します。', () => {
      Editor.advanced = !Editor.advanced;
      setStatus('info', Editor.advanced ? '高度なモードを有効にしました。危険な操作が表示されます。' : '高度なモードを無効にしました。');
      render();
    }));
  page.append(options);

  const data = card(cardHeader('データ'));
  data.append(h('div', {
    cls: 'small',
    text: '対応形式: Steam PC ネイティブセーブ (DATA.DAT, payload 0x31)。PS4 変換セーブ (0x2D) は非対応です。',
  }));
  page.append(data);

  const about = card(cardHeader('このアプリについて'));
  about.append(
    h('div', { text: `P5R Save Editor v${APP_VERSION} (web)` }),
    h('div', { cls: 'small', text: 'ペルソナ・スキル・装備・アイテム名は日本語表記です。' }),
    h('div', { cls: 'small', text: '実機でのロード確認は行っていません。初回は必ずバックアップを取り、ゲーム内でロードを確認してください。' }),
    h('div', { cls: 'small', text: '非公式ツールです。ATLUS / SEGA とは無関係です。Persona 5 Royal は ATLUS / SEGA の商標です。' }));
  page.append(about);
  return page;
}

function settingRow(on, label, detail, onClick) {
  const row = h('div', { cls: 'item-row', style: 'cursor:pointer;' },
    cbox(on),
    h('div', { style: 'display:flex; flex-direction:column; gap:1px;' },
      h('span', { text: label }),
      h('span', { cls: 'hint', text: detail })));
  row.addEventListener('click', onClick);
  return row;
}

// ---------- modals ----------

function modalOverlay(content, closeOnBackdrop) {
  const overlay = h('div', { cls: 'overlay' }, content);
  overlay.addEventListener('mousedown', (e) => {
    if (e.target === overlay && closeOnBackdrop) closeModal();
  });
  return overlay;
}

function renderModal() {
  const m = Editor.modal;
  if (!m) return null;
  if (m.kind === 'slots') return modalOverlay(slotsModal(), true);
  if (m.kind === 'confirm') return modalOverlay(confirmModal(m), true);
  if (m.kind === 'message') return modalOverlay(messageModal(m), true);
  if (m.kind === 'picker') return modalOverlay(pickerModal(), true);
  if (m.kind === 'persona') return modalOverlay(personaModal(), false);
  return null;
}

function slotsModal() {
  const card = h('div', { cls: 'modal wide' },
    h('h2', { text: 'セーブスロット' }),
    h('div', { cls: 'small', text: '読み込む DATA01〜DATA16 を選択してください。' }),
    h('div', { cls: 'divider' }));
  const list = h('div', { cls: 'scroll', style: 'max-height:52vh;' });
  const entries = [...Editor.slots.values()].sort((a, b) => a.name.localeCompare(b.name));
  if (!entries.length) {
    list.append(h('div', { cls: 'hint', text: 'スロットがありません。「ファイルを開く」かフォルダスキャンで読み込んでください。' }));
  }
  for (const entry of entries) {
    const current = Editor.doc && Editor.doc.slotName === entry.name;
    const row = h('div', { cls: `slot-row${current ? ' current' : ''}${entry.valid ? '' : ' invalid'}` },
      h('span', { text: entry.name, style: 'width:64px; flex:none;' }),
      h('span', { text: entry.label, style: 'flex:1; overflow:hidden; text-overflow:ellipsis; white-space:nowrap;' }),
      tag(entry.valid ? '読込可' : '不可', entry.valid ? 'ok' : ''));
    if (entry.valid) {
      row.addEventListener('click', () => requestLoadSlot(entry.name));
    }
    list.append(row);
  }
  card.append(list);
  card.append(h('div', { cls: 'foot' }, btn('閉じる', closeModal, 'ghost')));
  return card;
}

function confirmModal(m) {
  const card = h('div', { cls: 'modal' },
    h('h2', { text: m.title }),
    h('div', { text: m.message }));
  card.append(h('div', { cls: 'foot' },
    btn('キャンセル', () => {
      const cancel = m.cancelAction;
      closeModal();
      if (cancel) cancel();
    }),
    btn('実行', () => {
      const action = Editor.pending;
      Editor.pending = null;
      Editor.modal = null;
      if (action) action();
      render();
    }, 'primary')));
  return card;
}

function messageModal(m) {
  const card = h('div', { cls: 'modal' },
    h('h2', { text: m.title }),
    h('div', { text: m.message, style: 'white-space:pre-wrap;' }));
  card.append(h('div', { cls: 'foot' }, btn('OK', closeModal, 'primary')));
  return card;
}

function pickerModal() {
  const picker = Editor.picker;
  const title = picker.kind === 'persona' ? 'ペルソナを選択' : picker.kind === 'skill' ? 'スキルを選択' : '特性を選択';
  const rows = pickerRows();
  const pageSize = 60;
  const pages = Math.max(1, Math.ceil(rows.length / pageSize));
  const current = Math.min(picker.page, pages - 1);
  const start = current * pageSize;
  const end = Math.min(start + pageSize, rows.length);

  const card = h('div', { cls: 'modal wide' },
    h('h2', { text: title }));
  const input = h('input', { cls: 'plain', type: 'text', dataKey: 'picker-search' });
  input.value = picker.q;
  input.addEventListener('input', () => {
    picker.q = input.value;
    picker.page = 0;
    render();
  });
  card.append(h('span', { cls: 'search-box', style: 'width:100%;' },
    h('span', { cls: 'lbl', text: '検索' }), input));
  card.append(h('div', { cls: 'small', text: `${rows.length} 件` }));

  const list = h('div', { cls: 'scroll', style: 'max-height:44vh;' });
  for (let k = start; k < end; k++) {
    const [id, name] = rows[k];
    const row = h('div', { cls: 'pick-row' },
      h('span', { cls: 'pid', text: String(id).padStart(4, '0') }),
      h('span', { text: name }));
    row.addEventListener('click', () => pickerSelect(id));
    list.append(row);
  }
  card.append(list);
  card.append(h('div', { cls: 'foot' },
    btn('前へ', () => { picker.page = Math.max(0, current - 1); render(); }),
    h('span', { cls: 'small', text: `${current + 1} / ${pages}` }),
    btn('次へ', () => { picker.page = Math.min(pages - 1, current + 1); render(); }),
    h('span', { cls: 'spacer' }),
    btn('戻る', () => {
      Editor.picker = null;
      Editor.modal = { kind: 'persona' };
      render();
    }, 'ghost')));
  // focus the search box once inserted
  queueMicrotask(() => input.focus());
  return card;
}

function personaModal() {
  const p = Editor.persona;
  if (!p) return h('div', { cls: 'modal' }, h('h2', { text: 'エラー' }), h('div', { text: '編集対象のペルソナがありません。' }));
  const charName = characters[p.character]?.name ?? '';
  const name = personaName(p.draft.id);
  const traitLabel = traitName(p.draft.trait_id);

  const card = h('div', { cls: 'modal wide' },
    h('div', { cls: 'row' },
      h('h2', { text: `ペルソナ編集  /  ${charName}  /  枠 ${String(p.slot + 1).padStart(2, '0')}` }),
      h('span', { cls: 'spacer' }),
      h('span', { cls: 'small', text: `ID ${p.draft.id}  ${name}` })),
    h('div', { cls: 'divider' }));

  const body = h('div', { cls: 'scroll', style: 'max-height:calc(100vh - 200px); gap:8px;' });
  body.append(
    h('div', { cls: 'row' },
      h('span', { cls: 'field-label', text: 'ペルソナ' }),
      h('span', { text: `${p.draft.id}  ${name}` }),
      h('span', { cls: 'spacer' }),
      p.idLocked ? null : btn('選択', () => openPicker('persona', { type: 'personaId' }))),
    h('div', { cls: 'row' },
      labelledField('レベル', p.level, () => {
        const v = parseU8(p.level.v);
        return v !== null && v >= 1 && v <= 99 ? v : null;
      }, commitPersonaDraft, 62),
      labelledField('EXP', p.xp, () => {
        const v = parseU32(p.xp.v);
        return v !== null && v <= 2000000000 ? v : null;
      }, commitPersonaDraft, 96)),
    h('div', { cls: 'row' },
      h('span', { cls: 'field-label', text: '特性' }),
      h('span', { text: traitLabel }),
      h('span', { cls: 'spacer' }),
      btn('選択', () => openPicker('trait', { type: 'trait' }))),
    h('div', { cls: 'small', text: 'ステータス' }),
    h('div', { cls: 'row' },
      ...['力', '魔', '耐', '速', '運'].map((label, i) =>
        labelledField(label, p.stats[i], () => {
          const v = parseU8(p.stats[i].v);
          return v !== null && v <= 99 ? v : null;
        }, commitPersonaDraft, 54))),
    h('div', { cls: 'small', text: 'スキル' }));

  const skillList = h('div', { style: 'display:flex; flex-direction:column; gap:1px;' });
  for (let slot = 0; slot < 8; slot++) {
    const s = slot;
    skillList.append(h('div', { cls: 'row', style: 'min-height:26px;' },
      h('span', { cls: 'pid mono', style: 'width:22px;', text: String(s + 1).padStart(2, '0') }),
      h('span', { style: 'flex:1; overflow:hidden; text-overflow:ellipsis; white-space:nowrap;', text: skillName(p.draft.skills[s]) }),
      btn('変更', () => openPicker('skill', { type: 'skill', slot: s }), 'link')));
  }
  body.append(skillList);

  if (Editor.advanced && p.canClear) {
    body.append(h('div', { cls: 'divider' }));
    const clearRow = h('div', { cls: 'row' },
      btn(p.draft.clear ? '空にするのをやめる' : 'このスロットを空にする', () => {
        p.draft.clear = !p.draft.clear;
        render();
      }, 'danger'));
    if (p.draft.clear) clearRow.append(tag('適用で空になります', 'danger'));
    body.append(clearRow);
  }
  card.append(body);
  card.append(h('div', { cls: 'foot' },
    btn('キャンセル', () => {
      Editor.persona = null;
      Editor.picker = null;
      closeModal();
    }),
    btn('適用', applyPersona, 'primary')));
  return card;
}

// ---------- shell ----------

const PAGES = [
  ['basic', '基本'],
  ['confidants', 'コープ'],
  ['members', 'メンバー'],
  ['items', '所持品'],
  ['compendium', '図鑑'],
  ['backups', 'バックアップ'],
  ['settings', '設定'],
];

function pageHeader(title, desc) {
  return h('div', { cls: 'page-header' },
    h('h2', { text: title }),
    h('span', { cls: 'desc', text: desc }),
    h('span', { cls: 'spacer' }));
}

function emptyState() {
  const wrap = h('div', { cls: 'empty-state' },
    h('div', { text: 'DATA01〜DATA16 のいずれかを読み込んでください。' }),
    btn('ファイルを開く', () => fileInput.click(), 'primary'));
  return wrap;
}

let fileInput;

function render() {
  const app = document.getElementById('app');
  // Preserve content scroll and the focused control across rebuilds.
  const scrollEl = app.querySelector('.content');
  const scrollTop = scrollEl ? scrollEl.scrollTop : 0;
  const active = document.activeElement;
  let restore = null;
  if (active && active.tagName === 'INPUT' && app.contains(active) && active.dataset.fkey) {
    restore = { key: active.dataset.fkey, pos: active.selectionStart };
  }
  app.textContent = '';

  const sidebar = h('div', { cls: 'sidebar' },
    h('div', { cls: 'brand' },
      h('span', { cls: 'p5', text: 'P5R' }),
      h('span', { cls: 'sub', text: 'SAVE EDITOR' })),
    h('div', { style: 'height:4px;' }));
  for (const [key, label] of PAGES) {
    const item = h('div', { cls: `nav-item${Editor.page === key ? ' active' : ''}`, text: label });
    item.addEventListener('click', () => {
      if (Editor.page !== key) {
        Editor.page = key;
        if (key === 'items' && Editor.drafts.itemCategory !== currentCategory()?.key) syncItemDrafts();
      }
      render();
    });
    sidebar.append(item);
  }
  sidebar.append(h('div', { cls: 'spacer' }));
  const advToggle = checkToggle(Editor.advanced, '高度なモード', () => {
    Editor.advanced = !Editor.advanced;
    setStatus('info', Editor.advanced ? '高度なモードを有効にしました。危険な操作が表示されます。' : '高度なモードを無効にしました。');
    render();
  });
  advToggle.classList.add('adv-toggle');
  advToggle.classList.toggle('on', Editor.advanced);
  sidebar.append(advToggle);
  sidebar.append(h('div', { cls: 'version', text: `v${APP_VERSION}` }));

  const doc = Editor.doc;
  const slotLabelText = doc
    ? `${doc.slotName}  |  ${doc.header.fullName()}  |  ${formatDate(doc.header.day)}`
    : 'セーブ未読込';
  const topbar = h('div', { cls: 'topbar' },
    h('div', { cls: 'slot-chip' },
      h('span', { cls: 'label', text: slotLabelText }),
      h('span', { cls: 'hint', text: '変更' })),
    h('span', { cls: 'spacer' }));
  topbar.querySelector('.slot-chip').addEventListener('click', () => {
    Editor.modal = { kind: 'slots' };
    render();
  });
  if (doc) {
    topbar.append(
      doc.dirty ? tag('未保存', 'warn') : h('span', { cls: 'hint', text: '保存済み' }),
      btn('保存', saveDocument, 'primary'),
      btn('元に戻す', () => {
        if (doc.dirty) {
          requestConfirm('変更を元に戻す', '未保存の変更を破棄して、読み込み時の状態に戻しますか？', () => {
            doc.revert();
            syncAllDrafts();
            setStatus('info', '変更を元に戻しました。');
            render();
          });
        }
      }),
      btn('バックアップ', () =>
        requestConfirm('バックアップ', '編集前のオリジナルファイルをダウンロードしますか？', downloadOriginal)),
      btn('ファイルを開く', () => fileInput.click()));
    if (canFs) topbar.append(btn('フォルダから読込', scanFolder));
  } else {
    topbar.append(
      btn('ファイルを開く', () => fileInput.click(), 'primary'));
    if (canFs) topbar.append(btn('フォルダから読込', scanFolder));
  }

  const content = h('div', { cls: 'content' });
  if (!doc && !['backups', 'settings'].includes(Editor.page)) {
    content.append(h('div', { cls: 'page' }, emptyState()));
  } else {
    switch (Editor.page) {
      case 'basic': content.append(basicPage()); break;
      case 'confidants': content.append(confidantsPage()); break;
      case 'members': content.append(membersPage()); break;
      case 'items': content.append(itemsPage()); break;
      case 'compendium': content.append(compendiumPage()); break;
      case 'backups': content.append(backupsPage()); break;
      case 'settings': content.append(settingsPage()); break;
    }
  }
  content.scrollTop = scrollTop;

  const statusbar = h('div', { cls: 'statusbar' },
    h('span', { cls: `msg ${Editor.statusKind}`, text: Editor.status || '準備完了' }));
  if (Editor.advanced) statusbar.append(tag('高度', 'warn'));
  statusbar.append(h('span', { cls: 'small', text: `v${APP_VERSION}` }));

  const shell = h('div', { cls: 'shell' },
    sidebar,
    h('div', { cls: 'main' }, topbar, content, statusbar));

  const modal = renderModal();
  if (modal) shell.append(modal);
  app.append(shell);

  if (restore && restore.key) {
    const el = app.querySelector(`input[data-fkey="${restore.key}"]`);
    if (el) {
      el.focus();
      try { el.setSelectionRange(restore.pos, restore.pos); } catch { /* non-text input */ }
    }
  }
}

// ---------- boot ----------

fileInput = h('input', { type: 'file' });
fileInput.accept = '.DAT,.dat';
fileInput.multiple = true;
fileInput.style.display = 'none';
fileInput.addEventListener('change', async () => {
  const files = [...fileInput.files];
  fileInput.value = '';
  await openFiles(files);
});
document.body.append(fileInput);

window.addEventListener('beforeunload', (e) => {
  if (Editor.doc && Editor.doc.dirty) {
    e.preventDefault();
    e.returnValue = '';
  }
});

setStatus('info', 'P5R の DATA.DAT を開いてください。');
render();

// Exported for the headless smoke test; the UI itself uses module state only.
export { Editor };
Editor.__test = {
  load: (raw, name) => loadSlot(name, raw, null),
  render,
  validateAll,
  applyAll,
};
