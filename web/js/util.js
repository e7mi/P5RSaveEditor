// Little-endian readers and text helpers shared by the save modules.

const u8 = (b) => new Uint8Array(b);
const view = (b) => (b instanceof Uint8Array ? new DataView(b.buffer, b.byteOffset, b.byteLength) : new DataView(b));

export function readU16(buf, offset) {
  return view(buf).getUint16(offset, true);
}

export function readU32(buf, offset) {
  return view(buf).getUint32(offset, true);
}

export function writeU16(buf, offset, value) {
  view(buf).setUint16(offset, value, true);
}

export function writeU32(buf, offset, value) {
  view(buf).setUint32(offset, value, true);
}

export function align(value, alignment) {
  return (value + (alignment - 1)) & ~(alignment - 1);
}

export function toFullwidth(text) {
  return [...text].map((ch) => {
    const code = ch.codePointAt(0);
    return code >= 0x21 && code <= 0x7e ? String.fromCodePoint(code + 0xfee0) : ch;
  }).join('');
}

const decoder = new TextDecoder();
const encoder = new TextEncoder();

export function decodeText(raw) {
  const bytes = raw instanceof Uint8Array ? raw : u8(raw);
  const end = bytes.indexOf(0);
  return decoder.decode(end < 0 ? bytes : bytes.subarray(0, end));
}

// UTF-8-safe truncation: cut at the last boundary that still decodes cleanly.
export function truncateUtf8(text, length) {
  const data = encoder.encode(text);
  if (data.length <= length) {
    return data;
  }
  for (let end = length; end > 0; end--) {
    const slice = data.subarray(0, end);
    try {
      decoder.decode(slice, { fatal: true });
      return slice;
    } catch {
      // keep shrinking until the prefix is valid UTF-8
    }
  }
  return u8(0);
}

export function writeText(dst, text) {
  const target = dst instanceof Uint8Array ? dst : u8(dst);
  target.fill(0);
  target.set(truncateUtf8(text, target.length));
}

export function encodeText(text, length) {
  const out = u8(length);
  writeText(out, text);
  return out;
}
