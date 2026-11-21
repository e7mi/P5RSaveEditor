// Smoke test for web/js/app.js: minimal DOM shim, load a real save, render
// every page, and drive validate -> apply -> toBytes.

import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import assert from 'node:assert/strict';

class El {
  constructor(tag) {
    this.tagName = tag.toUpperCase();
    this.children = [];
    this.style = {};
    this.dataset = {};
    this.listeners = {};
    this.value = '';
    this.textContent = '';
    this.className = '';
    this.parentNode = null;
    this.classList = {
      _set: new Set(),
      add: (c) => this.classList._set.add(c),
      toggle: (c, on) => (on ? this.classList._set.add(c) : this.classList._set.delete(c)),
      contains: (c) => this.classList._set.has(c),
    };
  }
  append(...kids) {
    for (const k of kids.flat(Infinity)) {
      if (k === null || k === undefined) continue;
      const el = typeof k === 'string' ? Object.assign(new El('#text'), { textContent: k }) : k;
      el.parentNode = this;
      this.children.push(el);
    }
  }
  addEventListener(type, fn) {
    (this.listeners[type] ??= []).push(fn);
  }
  dispatch(type, event = {}) {
    for (const fn of this.listeners[type] ?? []) fn(event);
  }
  setAttribute(k, v) {
    this.dataset['attr-' + k] = v;
  }
  click() {
    this.dispatch('click');
  }
  querySelector(sel) {
    return this.querySelectorAll(sel)[0] ?? null;
  }
  querySelectorAll(sel) {
    const out = [];
    const match = (el) => {
      if (sel === '.content') return (el.className || '').split(' ').includes('content');
      if (sel.startsWith('input[data-fkey=')) {
        const key = sel.slice('input[data-fkey="'.length, -1);
        return el.tagName === 'INPUT' && el.dataset.fkey === key;
      }
      if (sel === '.slot-chip') return (el.className || '').split(' ').includes('slot-chip');
      return false;
    };
    const walk = (el) => {
      if (match(el)) out.push(el);
      for (const c of el.children) walk(c);
    };
    walk(this);
    return out;
  }
  contains(el) {
    for (let p = el; p; p = p.parentNode) if (p === this) return true;
    return false;
  }
  focus() {}
  setSelectionRange() {}
  remove() {
    if (this.parentNode) {
      const i = this.parentNode.children.indexOf(this);
      if (i >= 0) this.parentNode.children.splice(i, 1);
    }
  }
}

const appEl = new El('div');
appEl.id = 'app';
const bodyEl = new El('body');

globalThis.window = globalThis;
globalThis.document = {
  createElement: (tag) => new El(tag),
  getElementById: (id) => (id === 'app' ? appEl : null),
  body: bodyEl,
  activeElement: null,
};
globalThis.queueMicrotask = globalThis.queueMicrotask ?? ((f) => Promise.resolve().then(f));
window.addEventListener = () => {};
window.showDirectoryPicker = undefined; // feature-detect off for the test
window.showSaveFilePicker = undefined;
delete window.showDirectoryPicker;
delete window.showSaveFilePicker;

const { Editor } = await import('../web/js/app.js');

const REAL_SAVE = process.env.P5R_TEST_SAVE ?? 'H:/0001/games/persona/testsave/DATA05/DATA.DAT';

test('UI renders all pages with a real save', async () => {
  let raw;
  try {
    raw = new Uint8Array(readFileSync(REAL_SAVE));
  } catch {
    return;
  }
  await Editor.__test.load(raw, 'DATA05');
  assert.ok(Editor.doc);
  assert.equal(Editor.drafts.members.length, 10);
  assert.equal(Editor.drafts.confRank.length, 23);
  assert.equal(Editor.drafts.basic.social.length, 5);

  for (const page of ['basic', 'confidants', 'members', 'items', 'compendium', 'backups', 'settings']) {
    Editor.page = page;
    Editor.__test.render();
    assert.ok(appEl.children.length > 0, `page ${page} rendered`);
  }

  // save path: validate + apply + pack without touching handles
  assert.equal(Editor.__test.validateAll(), null);
  Editor.__test.applyAll();
  const packed = await Editor.doc.toBytes();
  assert.ok(packed.length > 0);

  // invalid money should fail validation and stay uncommitted
  const before = Editor.doc.money();
  Editor.drafts.basic.money.v = 'abc';
  assert.equal(Editor.__test.validateAll(), '所持金は 0〜9,999,999 の数値で入力してください。');
  assert.equal(Editor.doc.money(), before);
  Editor.drafts.basic.money.v = String(before);
});
