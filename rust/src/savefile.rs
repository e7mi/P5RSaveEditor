//! P5R (Steam) save container: AES-256-CBC, zlib, CRC-32/MPEG-2.

use std::io::Read;
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

use aes::Aes256;
use cbc::{Decryptor, Encryptor};
use cipher::block_padding::NoPadding;
use cipher::{BlockDecryptMut, BlockEncryptMut, KeyIvInit};
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use rand::rngs::OsRng;
use rand::RngCore;
use thiserror::Error;

use crate::key::CRYPT_KEY;

pub const DATA_SIZE: usize = 0x30720;

#[derive(Debug, Error)]
pub enum SaveError {
    #[error("{0}")]
    Message(String),
    #[error("CRC mismatch: {0}")]
    Crc(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

pub type SaveResult<T> = Result<T, SaveError>;

fn crc_table() -> [u32; 256] {
    let mut table = [0u32; 256];
    for (i, slot) in table.iter_mut().enumerate() {
        let mut crc = (i as u32) << 24;
        for _ in 0..8 {
            if crc & 0x8000_0000 != 0 {
                crc = (crc << 1) ^ 0x04C1_1DB7;
            } else {
                crc <<= 1;
            }
        }
        *slot = crc;
    }
    table
}

fn table() -> &'static [u32; 256] {
    static TABLE: OnceLock<[u32; 256]> = OnceLock::new();
    TABLE.get_or_init(crc_table)
}

pub fn calc_crc(buffer: &[u8]) -> u32 {
    let table = table();
    let mut init = 0xFFFF_FFFFu32;
    for &byte in buffer {
        init = (init << 8) ^ table[((byte as u32 ^ (init >> 24)) & 0xFF) as usize];
    }
    init
}

pub fn align(value: usize, alignment: usize) -> usize {
    (value + (alignment - 1)) & !(alignment - 1)
}

pub fn to_fullwidth(text: &str) -> String {
    text.chars()
        .map(|ch| {
            let code = ch as u32;
            if (0x21..=0x7E).contains(&code) {
                char::from_u32(code + 0xFEE0).unwrap_or(ch)
            } else {
                ch
            }
        })
        .collect()
}

#[derive(Clone, Debug)]
pub struct SaveHeader {
    pub playtime: u32,
    pub day: u16,
    pub time: u8,
    pub playthrough: u8,
    pub difficulty: u8,
    pub level: u8,
    pub clear: u8,
    pub fld_major: u8,
    pub fld_minor: u8,
    pub lang0: u8,
    pub lang1: u8,
    pub lname: String,
    pub fname: String,
    pub desc: String,
}

impl Default for SaveHeader {
    fn default() -> Self {
        Self {
            playtime: 0,
            day: 0,
            time: 0,
            playthrough: 1,
            difficulty: 2,
            level: 1,
            clear: 0,
            fld_major: 0,
            fld_minor: 0,
            lang0: 0xFF,
            lang1: 0,
            lname: String::new(),
            fname: String::new(),
            desc: String::new(),
        }
    }
}

impl SaveHeader {
    pub const SIZE: usize = 0x190;

    pub fn from_bytes(buffer: &[u8]) -> SaveResult<Self> {
        if buffer.len() < Self::SIZE {
            return Err(SaveError::Message(
                "header block is shorter than 0x190 bytes".into(),
            ));
        }
        Ok(Self {
            playtime: read_u32(buffer, 0),
            day: read_u16(buffer, 4),
            time: buffer[6],
            playthrough: buffer[7],
            difficulty: buffer[8],
            level: buffer[9],
            clear: buffer[10],
            fld_major: buffer[12],
            fld_minor: buffer[13],
            lang0: buffer[14],
            lang1: buffer[15],
            lname: decode_text(&buffer[16..80]),
            fname: decode_text(&buffer[80..144]),
            desc: decode_text(&buffer[144..400]),
        })
    }

    pub fn to_bytes(&self) -> [u8; Self::SIZE] {
        let mut out = [0u8; Self::SIZE];
        write_u32(&mut out, 0, self.playtime);
        write_u16(&mut out, 4, self.day);
        out[6] = self.time;
        out[7] = self.playthrough;
        out[8] = self.difficulty;
        out[9] = self.level;
        out[10] = self.clear;
        out[12] = self.fld_major;
        out[13] = self.fld_minor;
        out[14] = self.lang0;
        out[15] = self.lang1;
        write_text(&mut out[16..80], &self.lname);
        write_text(&mut out[80..144], &self.fname);
        write_text(&mut out[144..400], &self.desc);
        out
    }

    pub fn full_name(&self) -> String {
        format!("{} {}", self.lname, self.fname).trim().to_string()
    }

    pub fn playtime_minutes(&self) -> u32 {
        self.playtime / (30 * 60)
    }

    pub fn playtime_text(&self) -> String {
        let total_minutes = self.playtime_minutes();
        format!("{}時間{:02}分", total_minutes / 60, total_minutes % 60)
    }

    pub fn set_full_name(&mut self, name: &str) {
        let name = name.trim();
        if name.is_empty() {
            return;
        }
        let name = to_fullwidth(name);
        let (last, first) = match name.split_once(' ') {
            Some((last, first)) => (last.to_string(), first.to_string()),
            None => (name, String::new()),
        };
        self.lname = last;
        self.fname = first;
    }

    /// Rewrites only the date and time-of-day prefix of the save-select description.
    pub fn update_date_desc(&mut self, day: u16, time: u8) {
        let date = crate::calendar::format_date(day);
        let time = crate::calendar::time_desc_label(time);
        let mut lines: Vec<String> = self.desc.split('\n').map(str::to_string).collect();
        if lines.is_empty() {
            lines.push(String::new());
        }
        let rest = lines[0]
            .splitn(3, ' ')
            .nth(2)
            .unwrap_or("")
            .trim_start()
            .to_string();
        lines[0] = if rest.is_empty() {
            format!("{date} {time}")
        } else {
            format!("{date} {time} {rest}")
        };
        self.desc = lines.join("\n");
    }

    pub fn update_desc(
        &mut self,
        name: Option<&str>,
        level: Option<u8>,
        playtime: Option<u32>,
        difficulty: Option<u8>,
    ) {
        let mut lines: Vec<String> = self.desc.split('\n').map(str::to_string).collect();
        let name = name.unwrap_or("");
        let name = if name.is_empty() {
            self.full_name()
        } else {
            name.to_string()
        };
        let level = level.unwrap_or(self.level);
        let playtime = playtime.unwrap_or(self.playtime);
        let difficulty = difficulty.unwrap_or(self.difficulty);
        if lines.len() >= 2 && lines[1].starts_with("PLV") {
            lines[1] = format!("PLV {level} {name}").trim_end().to_string();
        }
        if lines.len() >= 3 && lines[2].starts_with("PLAY TIME") {
            let total_minutes = playtime / (30 * 60);
            lines[2] = format!(
                "PLAY TIME {}時間{}分",
                total_minutes / 60,
                total_minutes % 60
            );
        }
        if lines.len() >= 4 && lines[3].starts_with("難易度") {
            let diff = match difficulty {
                0 => "SAFETY",
                1 => "EASY",
                2 => "NORMAL",
                3 => "HARD",
                4 => "MERCILESS",
                _ => "EASY",
            };
            lines[3] = format!("難易度 {diff}");
        }
        self.desc = lines.join("\n");
    }
}

fn decode_text(raw: &[u8]) -> String {
    let end = raw.iter().position(|&b| b == 0).unwrap_or(raw.len());
    String::from_utf8_lossy(&raw[..end]).to_string()
}

fn truncate_utf8(text: &str, length: usize) -> Vec<u8> {
    let data = text.as_bytes();
    if data.len() <= length {
        return data.to_vec();
    }
    let mut end = length;
    while end > 0 {
        if std::str::from_utf8(&data[..end]).is_ok() {
            return data[..end].to_vec();
        }
        end -= 1;
    }
    Vec::new()
}

pub fn encode_text(text: &str, length: usize) -> Vec<u8> {
    let mut out = vec![0u8; length];
    write_text(&mut out, text);
    out
}

fn write_text(dst: &mut [u8], text: &str) {
    dst.fill(0);
    let data = truncate_utf8(text, dst.len());
    dst[..data.len()].copy_from_slice(&data);
}

pub fn read_u16(buf: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([buf[offset], buf[offset + 1]])
}

pub fn read_u32(buf: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        buf[offset],
        buf[offset + 1],
        buf[offset + 2],
        buf[offset + 3],
    ])
}

pub fn write_u16(buf: &mut [u8], offset: usize, value: u16) {
    buf[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}

pub fn write_u32(buf: &mut [u8], offset: usize, value: u32) {
    buf[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

#[derive(Clone, Debug)]
pub struct SaveContainer {
    pub header: SaveHeader,
    pub data: Vec<u8>,
    pub file_timestamp: u32,
    pub file_flags: u32,
    pub file_iv: [u8; 16],
    pub save_flags: u32,
    pub header_size: u16,
    pub header_size_comp: u16,
    pub data_size: u32,
    pub data_size_comp: u32,
    pub data_crc: u32,
    pub stored_file_crc: u32,
    pub stored_data_crc: u32,
    pub file_crc_ok: bool,
    pub data_crc_ok: bool,
    pub was_encrypted: bool,
}

impl Default for SaveContainer {
    fn default() -> Self {
        Self {
            header: SaveHeader::default(),
            data: Vec::new(),
            file_timestamp: 0,
            file_flags: 0,
            file_iv: [0; 16],
            save_flags: 0,
            header_size: 0x1D0,
            header_size_comp: 0,
            data_size: 0,
            data_size_comp: 0,
            data_crc: 0,
            stored_file_crc: 0,
            stored_data_crc: 0,
            file_crc_ok: true,
            data_crc_ok: true,
            was_encrypted: false,
        }
    }
}

impl SaveContainer {
    pub const MAGIC: &'static [u8; 4] = b"DATA";

    pub fn from_bytes(raw: &[u8], verify_crc: bool) -> SaveResult<Self> {
        let mut obj = Self::default();
        obj.parse(raw, verify_crc)?;
        Ok(obj)
    }

    fn parse(&mut self, raw: &[u8], verify_crc: bool) -> SaveResult<()> {
        if raw.len() < 0x40 {
            return Err(SaveError::Message(
                "ファイルが小さすぎます。P5R の DATA.DAT ではありません。".into(),
            ));
        }
        if &raw[0..4] != Self::MAGIC {
            return Err(SaveError::Message(
                "DATA マジックが見つかりません。P5R のセーブファイルではありません。".into(),
            ));
        }
        self.stored_file_crc = read_u32(raw, 4);
        self.file_timestamp = read_u32(raw, 8);
        self.file_flags = read_u32(raw, 12);
        self.file_iv.copy_from_slice(&raw[0x10..0x20]);
        self.was_encrypted = self.file_flags & 0x8000_0000 != 0;

        let calculated = calc_crc(&raw[0x8..]);
        self.file_crc_ok = calculated == self.stored_file_crc;
        if verify_crc && !self.file_crc_ok {
            return Err(SaveError::Crc(format!(
                "file stored={:08x} calculated={:08x}",
                self.stored_file_crc, calculated
            )));
        }

        let mut body = raw[0x20..].to_vec();
        if self.was_encrypted {
            if !body.len().is_multiple_of(16) {
                return Err(SaveError::Message("暗号化本体の長さが不正です。".into()));
            }
            let cipher = Decryptor::<Aes256>::new(&CRYPT_KEY.into(), &self.file_iv.into());
            let mut buffer = body;
            let plain = cipher
                .decrypt_padded_mut::<NoPadding>(&mut buffer)
                .map_err(|_| SaveError::Message("AES 復号に失敗しました。".into()))?
                .to_vec();
            body = plain;
            self.file_flags &= !0x8000_0000;
        }
        if body.len() < 0x40 {
            return Err(SaveError::Message("セーブ本体が短すぎます。".into()));
        }

        self.header_size = read_u16(&body, 0);
        self.header_size_comp = read_u16(&body, 2);
        self.data_size = read_u32(&body, 4);
        self.data_size_comp = read_u32(&body, 8);
        self.save_flags = read_u32(&body, 12);
        self.stored_data_crc = read_u32(&body, 16);

        let header_end = if self.save_flags & 1 != 0 {
            self.header_size_comp as usize
        } else {
            self.header_size as usize
        };
        if header_end < 0x20 || header_end > body.len() {
            return Err(SaveError::Message("ヘッダー領域が不正です。".into()));
        }
        let header_raw = if self.save_flags & 1 != 0 {
            let result = zlib_decompress(&body[0x20..header_end])?;
            self.save_flags &= !1;
            result
        } else {
            body[0x20..header_end].to_vec()
        };

        let data_offset = align(header_end, 16) - 0x20;
        if data_offset > body.len() {
            return Err(SaveError::Message("データ領域が不正です。".into()));
        }
        let data_raw = if self.save_flags & 2 != 0 {
            let end = data_offset
                .checked_add(self.data_size_comp as usize)
                .ok_or_else(|| SaveError::Message("データサイズが不正です。".into()))?;
            if end > body.len() {
                return Err(SaveError::Message("データ領域が不正です。".into()));
            }
            let result = zlib_decompress(&body[data_offset..end])?;
            self.save_flags &= !2;
            result
        } else {
            let end = data_offset
                .checked_add(self.data_size as usize)
                .ok_or_else(|| SaveError::Message("データサイズが不正です。".into()))?;
            if end > body.len() {
                return Err(SaveError::Message("データ領域が不正です。".into()));
            }
            body[data_offset..end].to_vec()
        };

        self.header = if header_raw.len() >= SaveHeader::SIZE {
            SaveHeader::from_bytes(&header_raw)?
        } else {
            SaveHeader::default()
        };
        self.data = data_raw;
        let calculated = calc_crc(&self.data);
        self.data_crc_ok = calculated == self.stored_data_crc;
        if verify_crc && !self.data_crc_ok {
            return Err(SaveError::Crc(format!(
                "data stored={:08x} calculated={:08x}",
                self.stored_data_crc, calculated
            )));
        }
        Ok(())
    }

    pub fn is_pc_payload(&self) -> bool {
        self.data.len() >= 4 && read_u32(&self.data, 0) == 0x31
    }

    pub fn pack(
        &mut self,
        compress: bool,
        encrypt: bool,
        update_timestamp: bool,
    ) -> SaveResult<Vec<u8>> {
        let header_bytes = self.header.to_bytes();
        let header_size = 0x40 + header_bytes.len();
        let mut header_size_comp = 0usize;
        let mut hs = header_size;
        let mut save_flags = self.save_flags & !0x3;
        let header_payload = if compress {
            let payload = zlib_compress(&header_bytes)?;
            save_flags |= 1;
            header_size_comp = 0x40 + payload.len();
            hs = header_size_comp;
            payload
        } else {
            header_bytes.to_vec()
        };

        let data_bytes = self.data.clone();
        let data_size = data_bytes.len();
        let mut data_size_comp = 0usize;
        let mut ds = data_size;
        let data_payload = if compress {
            let payload = zlib_compress(&data_bytes)?;
            save_flags |= 2;
            data_size_comp = payload.len();
            ds = data_size_comp;
            payload
        } else {
            data_bytes.clone()
        };
        let data_crc = calc_crc(&data_bytes);

        let mut body = Vec::with_capacity(0x20 + align(hs, 16) - 0x40 + align(ds, 16));
        body.extend_from_slice(&(header_size as u16).to_le_bytes());
        body.extend_from_slice(&(header_size_comp as u16).to_le_bytes());
        body.extend_from_slice(&(data_size as u32).to_le_bytes());
        body.extend_from_slice(&(data_size_comp as u32).to_le_bytes());
        body.extend_from_slice(&save_flags.to_le_bytes());
        body.extend_from_slice(&data_crc.to_le_bytes());
        body.extend_from_slice(&[0u8; 12]);
        let header_padded = align(hs, 16) - 0x40;
        body.extend_from_slice(&header_payload);
        body.resize(0x20 + header_padded, 0);
        body.extend_from_slice(&data_payload);
        let data_padded = align(ds, 16);
        body.resize(0x20 + header_padded + data_padded, 0);

        let mut file_flags = self.file_flags & !0x8000_0000;
        let mut iv = [0u8; 16];
        if encrypt {
            file_flags |= 0x8000_0000;
            OsRng.fill_bytes(&mut iv);
            if !body.len().is_multiple_of(16) {
                return Err(SaveError::Message(
                    "本体長が 16 の倍数ではありません。".into(),
                ));
            }
            let cipher = Encryptor::<Aes256>::new(&CRYPT_KEY.into(), &iv.into());
            let mut buffer = body;
            let length = buffer.len();
            let encrypted = cipher
                .encrypt_padded_mut::<NoPadding>(&mut buffer, length)
                .map_err(|_| SaveError::Message("AES 暗号化に失敗しました。".into()))?
                .to_vec();
            body = encrypted;
        }

        let timestamp = if update_timestamp {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs() as u32)
                .unwrap_or(self.file_timestamp)
        } else {
            self.file_timestamp
        };

        let mut outer = Vec::with_capacity(24 + body.len());
        outer.extend_from_slice(&timestamp.to_le_bytes());
        outer.extend_from_slice(&file_flags.to_le_bytes());
        outer.extend_from_slice(&iv);
        outer.extend_from_slice(&body);
        let file_crc = calc_crc(&outer);
        let mut packed = Vec::with_capacity(8 + outer.len());
        packed.extend_from_slice(Self::MAGIC);
        packed.extend_from_slice(&file_crc.to_le_bytes());
        packed.extend_from_slice(&outer);

        self.file_timestamp = timestamp;
        self.file_flags = file_flags;
        self.file_iv = iv;
        self.save_flags = save_flags;
        self.header_size = header_size as u16;
        self.header_size_comp = header_size_comp as u16;
        self.data_size = data_size as u32;
        self.data_size_comp = data_size_comp as u32;
        self.data_crc = data_crc;
        self.stored_file_crc = file_crc;
        self.stored_data_crc = data_crc;
        self.file_crc_ok = true;
        self.data_crc_ok = true;
        self.was_encrypted = encrypt;
        Ok(packed)
    }
}

fn zlib_decompress(data: &[u8]) -> SaveResult<Vec<u8>> {
    let mut decoder = ZlibDecoder::new(data);
    let mut out = Vec::new();
    decoder
        .read_to_end(&mut out)
        .map_err(|err| SaveError::Message(format!("zlib 展開に失敗しました: {err}")))?;
    Ok(out)
}

fn zlib_compress(data: &[u8]) -> SaveResult<Vec<u8>> {
    use std::io::Write;
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::best());
    encoder
        .write_all(data)
        .map_err(|err| SaveError::Message(format!("zlib 圧縮に失敗しました: {err}")))?;
    encoder
        .finish()
        .map_err(|err| SaveError::Message(format!("zlib 圧縮に失敗しました: {err}")))
}
