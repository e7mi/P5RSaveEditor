// P5R (Steam) save container: AES-256-CBC, zlib, CRC-32/MPEG-2.

import { align, decodeText, readU16, readU32, toFullwidth, writeText, writeU16, writeU32 } from './util.js';
import { formatDate, timeDescLabel } from './calendar.js';
import { aes256Cbc } from './aes.js';

// P5R Steam save AES-256 key (from fiber-saveutil).
const CRYPT_KEY = new Uint8Array([
  0xde, 0x53, 0x99, 0x4b, 0x49, 0x18, 0x4a, 0x83, 0x8e, 0xb6, 0x40, 0xb8, 0x73,
  0xb2, 0x03, 0x7e, 0xf3, 0x57, 0x9f, 0x12, 0x29, 0xad, 0x43, 0xd3, 0x95, 0x41,
  0x95, 0x0b, 0x7c, 0x81, 0x24, 0x5d,
]);

export const DATA_SIZE = 0x30720;
export const MAGIC = [0x44, 0x41, 0x54, 0x41]; // "DATA"

export class SaveError extends Error {}

const crcTable = (() => {
  const table = new Uint32Array(256);
  for (let i = 0; i < 256; i++) {
    let crc = i << 24;
    for (let bit = 0; bit < 8; bit++) {
      crc = (crc & 0x80000000) !== 0 ? ((crc << 1) ^ 0x04c11db7) >>> 0 : (crc << 1) >>> 0;
    }
    table[i] = crc;
  }
  return table;
})();

export function calcCrc(buffer) {
  let init = 0xffffffff;
  for (let i = 0; i < buffer.length; i++) {
    init = ((init << 8) ^ crcTable[((buffer[i] ^ (init >>> 24)) & 0xff)]) >>> 0;
  }
  return init >>> 0;
}

async function zlibDecompress(data) {
  const stream = new DecompressionStream('deflate');
  const writer = stream.writable.getWriter();
  writer.write(data);
  writer.close();
  const response = new Response(stream.readable);
  return new Uint8Array(await response.arrayBuffer());
}

async function zlibCompress(data) {
  const stream = new CompressionStream('deflate');
  const writer = stream.writable.getWriter();
  writer.write(data);
  writer.close();
  const response = new Response(stream.readable);
  return new Uint8Array(await response.arrayBuffer());
}

export class SaveHeader {
  static SIZE = 0x190;

  static defaults() {
    return {
      playtime: 0,
      day: 0,
      time: 0,
      playthrough: 1,
      difficulty: 2,
      level: 1,
      clear: 0,
      fld_major: 0,
      fld_minor: 0,
      lang0: 0xff,
      lang1: 0,
      lname: '',
      fname: '',
      desc: '',
    };
  }

  constructor(fields = {}) {
    Object.assign(this, SaveHeader.defaults(), fields);
  }

  static fromBytes(buffer) {
    if (buffer.length < SaveHeader.SIZE) {
      throw new SaveError('header block is shorter than 0x190 bytes');
    }
    return new SaveHeader({
      playtime: readU32(buffer, 0),
      day: readU16(buffer, 4),
      time: buffer[6],
      playthrough: buffer[7],
      difficulty: buffer[8],
      level: buffer[9],
      clear: buffer[10],
      fld_major: buffer[12],
      fld_minor: buffer[13],
      lang0: buffer[14],
      lang1: buffer[15],
      lname: decodeText(buffer.subarray(16, 80)),
      fname: decodeText(buffer.subarray(80, 144)),
      desc: decodeText(buffer.subarray(144, 400)),
    });
  }

  toBytes() {
    const out = new Uint8Array(SaveHeader.SIZE);
    writeU32(out, 0, this.playtime);
    writeU16(out, 4, this.day);
    out[6] = this.time;
    out[7] = this.playthrough;
    out[8] = this.difficulty;
    out[9] = this.level;
    out[10] = this.clear;
    out[12] = this.fld_major;
    out[13] = this.fld_minor;
    out[14] = this.lang0;
    out[15] = this.lang1;
    writeText(out.subarray(16, 80), this.lname);
    writeText(out.subarray(80, 144), this.fname);
    writeText(out.subarray(144, 400), this.desc);
    return out;
  }

  fullName() {
    return `${this.lname} ${this.fname}`.trim();
  }

  playtimeMinutes() {
    return Math.floor(this.playtime / (30 * 60));
  }

  setFullName(name) {
    const trimmed = name.trim();
    if (!trimmed) {
      return;
    }
    const widened = toFullwidth(trimmed);
    const space = widened.indexOf(' ');
    this.lname = space < 0 ? widened : widened.slice(0, space);
    this.fname = space < 0 ? '' : widened.slice(space + 1);
  }

  // Rewrites only the date and time-of-day prefix of the save-select description.
  updateDateDesc(day, time) {
    const date = formatDate(day);
    const timeLabel = timeDescLabel(time);
    const lines = this.desc.split('\n');
    const rest = lines[0].split(' ', 3)[2]?.trimStart() ?? '';
    lines[0] = rest ? `${date} ${timeLabel} ${rest}` : `${date} ${timeLabel}`;
    this.desc = lines.join('\n');
  }

  updateDesc(level, playtime, difficulty) {
    const lines = this.desc.split('\n');
    const name = this.fullName();
    const levelValue = level ?? this.level;
    const playtimeValue = playtime ?? this.playtime;
    const difficultyValue = difficulty ?? this.difficulty;
    if (lines.length >= 2 && lines[1].startsWith('PLV')) {
      lines[1] = `PLV ${levelValue} ${name}`.trimEnd();
    }
    if (lines.length >= 3 && lines[2].startsWith('PLAY TIME')) {
      const total = Math.floor(playtimeValue / (30 * 60));
      lines[2] = `PLAY TIME ${Math.floor(total / 60)}時間${total % 60}分`;
    }
    if (lines.length >= 4 && lines[3].startsWith('難易度')) {
      const labels = ['SAFETY', 'EASY', 'NORMAL', 'HARD', 'MERCILESS'];
      lines[3] = `難易度 ${labels[difficultyValue] ?? 'EASY'}`;
    }
    this.desc = lines.join('\n');
  }
}

export class SaveContainer {
  constructor(fields = {}) {
    Object.assign(
      this,
      {
        header: new SaveHeader(),
        data: new Uint8Array(0),
        file_timestamp: 0,
        file_flags: 0,
        file_iv: new Uint8Array(16),
        save_flags: 0,
        header_size: 0x1d0,
        header_size_comp: 0,
        data_size: 0,
        data_size_comp: 0,
        data_crc: 0,
        stored_file_crc: 0,
        stored_data_crc: 0,
        file_crc_ok: true,
        data_crc_ok: true,
        was_encrypted: false,
      },
      fields
    );
  }

  isPcPayload() {
    return this.data.length >= 4 && readU32(this.data, 0) === 0x31;
  }

  static async fromBytes(raw, verifyCrc = false) {
    const container = new SaveContainer();
    await container.parse(raw, verifyCrc);
    return container;
  }

  async parse(raw, verifyCrc) {
    if (raw.length < 0x40) {
      throw new SaveError('ファイルが小さすぎます。P5R の DATA.DAT ではありません。');
    }
    for (let i = 0; i < 4; i++) {
      if (raw[i] !== MAGIC[i]) {
        throw new SaveError('DATA マジックが見つかりません。P5R のセーブファイルではありません。');
      }
    }
    this.stored_file_crc = readU32(raw, 4);
    this.file_timestamp = readU32(raw, 8);
    this.file_flags = readU32(raw, 12);
    this.file_iv = raw.slice(0x10, 0x20);
    this.was_encrypted = (this.file_flags & 0x80000000) !== 0;

    const calculated = calcCrc(raw.subarray(0x8));
    this.file_crc_ok = calculated === this.stored_file_crc;
    if (verifyCrc && !this.file_crc_ok) {
      throw new SaveError(
        `file stored=${this.stored_file_crc.toString(16).padStart(8, '0')} calculated=${calculated.toString(16).padStart(8, '0')}`
      );
    }

    let body = raw.slice(0x20);
    if (this.was_encrypted) {
      if (body.length % 16 !== 0) {
        throw new SaveError('暗号化本体の長さが不正です。');
      }
      body = aes256Cbc(CRYPT_KEY, this.file_iv, body, true);
      this.file_flags &= ~0x80000000;
    }
    if (body.length < 0x40) {
      throw new SaveError('セーブ本体が短すぎます。');
    }

    this.header_size = readU16(body, 0);
    this.header_size_comp = readU16(body, 2);
    this.data_size = readU32(body, 4);
    this.data_size_comp = readU32(body, 8);
    this.save_flags = readU32(body, 12);
    this.stored_data_crc = readU32(body, 16);

    const headerEnd = (this.save_flags & 1) !== 0 ? this.header_size_comp : this.header_size;
    if (headerEnd < 0x20 || headerEnd > body.length) {
      throw new SaveError('ヘッダー領域が不正です。');
    }
    // header_size(_comp) counts from the file start including the 0x20 body
    // header, so the payload inside `body` ends 0x20 earlier.
    const headerRaw =
      (this.save_flags & 1) !== 0
        ? await zlibDecompress(body.subarray(0x20, headerEnd - 0x20))
        : body.subarray(0x20, headerEnd - 0x20);
    if ((this.save_flags & 1) !== 0) {
      this.save_flags &= ~1;
    }

    const dataOffset = align(headerEnd, 16) - 0x20;
    if (dataOffset > body.length) {
      throw new SaveError('データ領域が不正です。');
    }
    const dataRaw =
      (this.save_flags & 2) !== 0
        ? await zlibDecompress(body.subarray(dataOffset, dataOffset + this.data_size_comp))
        : body.subarray(dataOffset, dataOffset + this.data_size);
    if ((this.save_flags & 2) !== 0) {
      this.save_flags &= ~2;
    }

    this.header =
      headerRaw.length >= SaveHeader.SIZE ? SaveHeader.fromBytes(headerRaw) : new SaveHeader();
    this.data = dataRaw;
    const dataCalculated = calcCrc(this.data);
    this.data_crc_ok = dataCalculated === this.stored_data_crc;
    if (verifyCrc && !this.data_crc_ok) {
      throw new SaveError(
        `data stored=${this.stored_data_crc.toString(16).padStart(8, '0')} calculated=${dataCalculated.toString(16).padStart(8, '0')}`
      );
    }
  }

  async pack(compress = true, encrypt = true, updateTimestamp = true) {
    const headerBytes = this.header.toBytes();
    const headerSize = 0x40 + headerBytes.length;
    let headerSizeComp = 0;
    let hs = headerSize;
    const saveFlags0 = this.save_flags & ~0x3;
    let saveFlags = saveFlags0;
    let headerPayload;
    if (compress) {
      headerPayload = await zlibCompress(headerBytes);
      saveFlags |= 1;
      headerSizeComp = 0x40 + headerPayload.length;
      hs = headerSizeComp;
    } else {
      headerPayload = headerBytes;
    }

    const dataBytes = this.data;
    const dataSize = dataBytes.length;
    let dataSizeComp = 0;
    let ds = dataSize;
    let dataPayload;
    if (compress) {
      dataPayload = await zlibCompress(dataBytes);
      saveFlags |= 2;
      dataSizeComp = dataPayload.length;
      ds = dataSizeComp;
    } else {
      dataPayload = dataBytes;
    }
    const dataCrc = calcCrc(dataBytes);

    const body = new Uint8Array(0x20 + align(hs, 16) - 0x40 + align(ds, 16));
    const bodyView = new DataView(body.buffer);
    bodyView.setUint16(0, headerSize, true);
    bodyView.setUint16(2, headerSizeComp, true);
    bodyView.setUint32(4, dataSize, true);
    bodyView.setUint32(8, dataSizeComp, true);
    bodyView.setUint32(12, saveFlags, true);
    bodyView.setUint32(16, dataCrc, true);
    const headerPadded = align(hs, 16) - 0x40;
    body.set(headerPayload, 0x20);
    body.set(dataPayload, 0x20 + headerPadded);

    let fileFlags = this.file_flags & ~0x80000000;
    const iv = new Uint8Array(16);
    let outerBody = body;
    if (encrypt) {
      fileFlags |= 0x80000000;
      crypto.getRandomValues(iv);
      if (body.length % 16 !== 0) {
        throw new SaveError('本体長が 16 の倍数ではありません。');
      }
      outerBody = aes256Cbc(CRYPT_KEY, iv, body, false);
    }

    const timestamp = updateTimestamp ? Math.floor(Date.now() / 1000) : this.file_timestamp;

    const outer = new Uint8Array(24 + outerBody.length);
    const outerView = new DataView(outer.buffer);
    outerView.setUint32(0, timestamp, true);
    outerView.setUint32(4, fileFlags >>> 0, true);
    outer.set(iv, 8);
    outer.set(outerBody, 24);
    const fileCrc = calcCrc(outer);
    const packed = new Uint8Array(8 + outer.length);
    packed.set(MAGIC, 0);
    new DataView(packed.buffer).setUint32(4, fileCrc, true);
    packed.set(outer, 8);

    this.file_timestamp = timestamp;
    this.file_flags = fileFlags;
    this.file_iv = iv;
    this.save_flags = saveFlags;
    this.header_size = headerSize;
    this.header_size_comp = headerSizeComp;
    this.data_size = dataSize;
    this.data_size_comp = dataSizeComp;
    this.data_crc = dataCrc;
    this.stored_file_crc = fileCrc;
    this.stored_data_crc = dataCrc;
    this.file_crc_ok = true;
    this.data_crc_ok = true;
    this.was_encrypted = encrypt;
    return packed;
  }
}
