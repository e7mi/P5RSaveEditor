// AES-256-CBC with no padding. The P5R save body is always 16-aligned, so
// WebCrypto's mandatory PKCS#7 handling cannot be used; this is a compact
// table-driven AES-256 instead.

const SBOX = new Uint8Array(256);
const INV_SBOX = new Uint8Array(256);
const RCON = new Uint8Array(15);

(function init() {
  // Multiplicative inverse in GF(2^8), then the affine transform for SBOX.
  const exp = new Uint8Array(256);
  const log = new Uint8Array(256);
  let x = 1;
  for (let i = 0; i < 255; i++) {
    exp[i] = x;
    log[x] = i;
    x ^= x << 1;
    if (x & 0x100) x ^= 0x11b;
  }
  exp[255] = exp[0];
  for (let i = 0; i < 256; i++) {
    const inv = i === 0 ? 0 : exp[(255 - log[i]) % 255];
    let s = inv;
    s ^= (inv << 1) | (inv >>> 7);
    s ^= (inv << 2) | (inv >>> 6);
    s ^= (inv << 3) | (inv >>> 5);
    s ^= (inv << 4) | (inv >>> 4);
    SBOX[i] = s ^ 0x63;
  }
  for (let i = 0; i < 256; i++) {
    INV_SBOX[SBOX[i]] = i;
  }
  let r = 1;
  for (let i = 0; i < 14; i++) {
    RCON[i + 1] = r;
    r = (r << 1) ^ (r & 0x80 ? 0x1b : 0);
  }
})();

const MUL2 = new Uint8Array(256);
const MUL3 = new Uint8Array(256);
const MUL9 = new Uint8Array(256);
const MUL11 = new Uint8Array(256);
const MUL13 = new Uint8Array(256);
const MUL14 = new Uint8Array(256);

(function initMix() {
  const mul = (a, b) => {
    let p = 0;
    for (let i = 0; i < 8; i++) {
      if (b & 1) p ^= a;
      a = (a << 1) ^ (a & 0x80 ? 0x1b : 0);
      b >>>= 1;
    }
    return p & 0xff;
  };
  for (let i = 0; i < 256; i++) {
    MUL2[i] = mul(i, 2);
    MUL3[i] = mul(i, 3);
    MUL9[i] = mul(i, 9);
    MUL11[i] = mul(i, 11);
    MUL13[i] = mul(i, 13);
    MUL14[i] = mul(i, 14);
  }
})();

function keyExpansion(key) {
  // 15 round keys of 16 bytes for AES-256.
  const expanded = new Uint8Array(240);
  expanded.set(key);
  let bytes = 32;
  let i = 1; // rcon index: first expansion word uses RCON[1]
  const t = new Uint8Array(4);
  while (bytes < 240) {
    t.set(expanded.subarray(bytes - 4, bytes));
    if (bytes % 32 === 0) {
      const rot = [SBOX[t[1]], SBOX[t[2]], SBOX[t[3]], SBOX[t[0]]];
      rot[0] ^= RCON[i++];
      t.set(rot);
    } else if (bytes % 32 === 16) {
      t.set([SBOX[t[0]], SBOX[t[1]], SBOX[t[2]], SBOX[t[3]]]);
    }
    for (let j = 0; j < 4; j++) {
      expanded[bytes] = expanded[bytes - 32] ^ t[j];
      bytes++;
    }
  }
  return expanded;
}

function encryptBlock(roundKeys, input, output, offset) {
  const s = new Uint8Array(16);
  for (let i = 0; i < 16; i++) s[i] = input[i] ^ roundKeys[i];
  let rk = 16;
  for (let round = 1; round <= 14; round++) {
    for (let i = 0; i < 16; i++) s[i] = SBOX[s[i]];
    // ShiftRows: state is column-major s[row + 4*col]
    const t = s.slice();
    s[1] = t[5]; s[5] = t[9]; s[9] = t[13]; s[13] = t[1];
    s[2] = t[10]; s[6] = t[14]; s[10] = t[2]; s[14] = t[6];
    s[3] = t[15]; s[7] = t[3]; s[11] = t[7]; s[15] = t[11];
    if (round < 14) {
      for (let c = 0; c < 4; c++) {
        const i = c * 4;
        const a0 = s[i], a1 = s[i + 1], a2 = s[i + 2], a3 = s[i + 3];
        s[i] = MUL2[a0] ^ MUL3[a1] ^ a2 ^ a3;
        s[i + 1] = a0 ^ MUL2[a1] ^ MUL3[a2] ^ a3;
        s[i + 2] = a0 ^ a1 ^ MUL2[a2] ^ MUL3[a3];
        s[i + 3] = MUL3[a0] ^ a1 ^ a2 ^ MUL2[a3];
      }
    }
    for (let i = 0; i < 16; i++) s[i] ^= roundKeys[rk + i];
    rk += 16;
  }
  output.set(s, offset);
}

function decryptBlock(roundKeys, input, output, offset) {
  const s = new Uint8Array(16);
  for (let i = 0; i < 16; i++) s[i] = input[i] ^ roundKeys[224 + i];
  for (let round = 14; round >= 1; round--) {
    const rk = (round - 1) * 16;
    // InvShiftRows
    const t = s.slice();
    s[1] = t[13]; s[5] = t[1]; s[9] = t[5]; s[13] = t[9];
    s[2] = t[10]; s[6] = t[14]; s[10] = t[2]; s[14] = t[6];
    s[3] = t[7]; s[7] = t[11]; s[11] = t[15]; s[15] = t[3];
    for (let i = 0; i < 16; i++) s[i] = INV_SBOX[s[i]] ^ roundKeys[rk + i];
    if (round > 1) {
      for (let c = 0; c < 4; c++) {
        const i = c * 4;
        const a0 = s[i], a1 = s[i + 1], a2 = s[i + 2], a3 = s[i + 3];
        s[i] = MUL14[a0] ^ MUL11[a1] ^ MUL13[a2] ^ MUL9[a3];
        s[i + 1] = MUL9[a0] ^ MUL14[a1] ^ MUL11[a2] ^ MUL13[a3];
        s[i + 2] = MUL13[a0] ^ MUL9[a1] ^ MUL14[a2] ^ MUL11[a3];
        s[i + 3] = MUL11[a0] ^ MUL13[a1] ^ MUL9[a2] ^ MUL14[a3];
      }
    }
  }
  output.set(s, offset);
}

export function aes256Cbc(key, iv, data, decrypt = false) {
  if (data.length % 16 !== 0) {
    throw new Error('AES-CBC input must be a multiple of 16 bytes');
  }
  const roundKeys = keyExpansion(key);
  const out = new Uint8Array(data.length);
  let chain = iv.slice();
  if (!decrypt) {
    for (let offset = 0; offset < data.length; offset += 16) {
      const block = data.subarray(offset, offset + 16);
      for (let i = 0; i < 16; i++) block[i] ^= chain[i];
      encryptBlock(roundKeys, block, out, offset);
      chain = out.subarray(offset, offset + 16);
    }
  } else {
    for (let offset = 0; offset < data.length; offset += 16) {
      const block = data.subarray(offset, offset + 16);
      decryptBlock(roundKeys, block, out, offset);
      for (let i = 0; i < 16; i++) out[offset + i] ^= chain[i];
      chain = block;
    }
  }
  return out;
}
