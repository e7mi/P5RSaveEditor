use std::path::{Path, PathBuf};

use rune::{col, row, spacer, text, Color, Cx, El, ScrollSpec, TextEdit};

use crate::backups::{BackupManager, BackupRecord};
use crate::calendar;
use crate::gamedata::{PersonaEdit, SaveDocument, MONEY_MAX};
use crate::input;
use crate::paths::{
    candidate_save_dirs, clean_path, default_save_dir, display_path, is_game_running,
    load_settings, local_app_dir, open_in_explorer, save_settings, slot_files, Settings,
    APP_VERSION,
};
use crate::savefile::SaveContainer;
use crate::tables::tables;
use crate::ui::{self, Metrics};
use crate::views;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page {
    Basic,
    Confidants,
    Members,
    Inventory,
    Compendium,
    Backups,
    Settings,
}

impl Page {
    pub const ALL: [Page; 7] = [
        Page::Basic,
        Page::Confidants,
        Page::Members,
        Page::Inventory,
        Page::Compendium,
        Page::Backups,
        Page::Settings,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Page::Basic => "基本",
            Page::Confidants => "コープ",
            Page::Members => "メンバー",
            Page::Inventory => "所持品",
            Page::Compendium => "図鑑",
            Page::Backups => "バックアップ",
            Page::Settings => "設定",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatusKind {
    Info,
    Error,
    Success,
}

#[derive(Clone, Debug)]
pub struct SlotSummary {
    pub name: String,
    pub path: PathBuf,
    pub valid: bool,
    pub label: String,
    pub mtime: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PickerKind {
    Persona,
    Skill,
    Trait,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PickerTarget {
    PersonaId,
    Trait,
    Skill(usize),
}

pub struct Picker {
    pub kind: PickerKind,
    pub target: PickerTarget,
    pub page: usize,
    pub search: TextEdit,
}

pub struct Field {
    pub edit: TextEdit,
}

impl Field {
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            edit: TextEdit::new(value),
        }
    }

    pub fn set(&mut self, value: impl Into<String>) {
        let value = value.into();
        if self.edit.text != value {
            self.edit.replace_text(value);
        }
    }

    pub fn text(&self) -> &str {
        &self.edit.text
    }

    pub fn is_focused(&self) -> bool {
        self.edit.focused
    }
}

pub struct BasicDrafts {
    pub name: Field,
    pub money: Field,
    pub playtime: Field,
    pub date: Field,
    pub social: Vec<Field>,
}

impl BasicDrafts {
    fn new() -> Self {
        Self {
            name: Field::new(""),
            money: Field::new("0"),
            playtime: Field::new("0:00"),
            date: Field::new("4/1"),
            social: Vec::new(),
        }
    }
}

pub struct ConfidantDrafts {
    pub rank: Vec<Field>,
    pub points: Vec<Field>,
}

impl ConfidantDrafts {
    fn new() -> Self {
        Self {
            rank: Vec::new(),
            points: Vec::new(),
        }
    }
}

pub struct MemberDrafts {
    pub hp: Field,
    pub sp: Field,
    pub level: Field,
    pub xp: Field,
}

impl MemberDrafts {
    fn new() -> Self {
        Self {
            hp: Field::new("0"),
            sp: Field::new("0"),
            level: Field::new("1"),
            xp: Field::new("0"),
        }
    }
}

pub struct PersonaEditor {
    pub character: usize,
    pub slot: usize,
    pub draft: PersonaEdit,
    pub id_locked: bool,
    pub can_clear: bool,
    pub level: Field,
    pub xp: Field,
    pub stats: [Field; 5],
}

#[derive(Clone, Debug)]
pub enum Modal {
    Slots,
    Confirm,
    Message(String, String),
    Persona,
    Picker,
}

#[derive(Clone, Debug)]
pub enum Action {
    Save,
    Revert,
    LoadSlot(PathBuf),
    ChangeFolder(PathBuf),
    AutoDetect,
    RestoreBackup(usize),
    CreateBackup,
    MaxMoney,
    MaxSocial,
    MaxConfidants,
    AllCompendium,
    ClearCompendium,
    MaxCategory(String),
    ClearCategory(String),
}

pub struct Editor {
    pub settings: Settings,
    pub save_dir: PathBuf,
    pub slots: Vec<SlotSummary>,
    pub doc: Option<SaveDocument>,
    pub page: Page,
    pub status: String,
    pub status_kind: StatusKind,
    pub backups: Vec<BackupRecord>,
    pub modal: Option<Modal>,
    pub pending: Option<Action>,
    pub confirm_title: String,
    pub confirm_message: String,
    pub selected_character: usize,
    pub selected_category: usize,
    pub item_page: usize,
    pub owned_only: bool,
    pub item_search: TextEdit,
    pub compendium_page: usize,
    pub compendium_search: TextEdit,
    pub confidant_search: TextEdit,
    pub basic: BasicDrafts,
    pub confidants: ConfidantDrafts,
    pub members: Vec<MemberDrafts>,
    pub item_fields: Vec<Field>,
    pub item_fields_category: String,
    pub date_time: u8,
    pub persona: Option<PersonaEditor>,
    pub picker: Option<Picker>,
}

impl Default for Editor {
    fn default() -> Self {
        Self::new()
    }
}

impl Editor {
    pub fn new() -> Self {
        let settings = load_settings();
        let save_dir = if !settings.save_dir.is_empty() {
            clean_path(Path::new(&settings.save_dir))
        } else {
            PathBuf::from(default_save_dir())
        };
        let slots = scan_slots(&save_dir);
        let mut editor = Self {
            settings,
            save_dir,
            slots,
            doc: None,
            page: Page::Basic,
            status: String::new(),
            status_kind: StatusKind::Info,
            backups: Vec::new(),
            modal: None,
            pending: None,
            confirm_title: String::new(),
            confirm_message: String::new(),
            selected_character: 0,
            selected_category: 0,
            item_page: 0,
            owned_only: false,
            item_search: TextEdit::new(""),
            compendium_page: 0,
            compendium_search: TextEdit::new(""),
            confidant_search: TextEdit::new(""),
            basic: BasicDrafts::new(),
            confidants: ConfidantDrafts::new(),
            members: Vec::new(),
            item_fields: Vec::new(),
            item_fields_category: String::new(),
            date_time: 4,
            persona: None,
            picker: None,
        };
        let target = editor
            .slots
            .iter()
            .find(|slot| slot.name == editor.settings.last_slot && slot.valid)
            .or_else(|| {
                editor
                    .slots
                    .iter()
                    .filter(|slot| slot.valid)
                    .max_by_key(|slot| slot.mtime)
            })
            .map(|slot| slot.path.clone());
        if let Some(path) = target {
            editor.load_slot(&path);
        } else if editor.save_dir.as_os_str().is_empty() {
            editor.page = Page::Settings;
            editor.set_status(
                StatusKind::Error,
                "セーブフォルダが見つかりません。設定から savedata フォルダを選択してください。",
            );
        } else {
            editor.page = Page::Settings;
            editor.set_status(
                StatusKind::Error,
                "有効なセーブが見つかりません。フォルダを確認してください。",
            );
        }
        editor.refresh_backups();
        editor
    }

    pub fn set_status(&mut self, kind: StatusKind, message: impl Into<String>) {
        self.status_kind = kind;
        self.status = message.into();
    }

    pub fn set_page(&mut self, page: Page) {
        if self.page != page {
            self.sync_all_drafts();
        }
        self.page = page;
        if page == Page::Inventory {
            self.ensure_item_fields();
        }
    }

    pub fn advanced(&self) -> bool {
        self.settings.advanced
    }

    pub fn toggle_advanced(&mut self) {
        self.settings.advanced = !self.settings.advanced;
        if !self.settings.advanced {
            self.settings.allow_save_while_running = false;
        }
        save_settings(&self.settings);
        let message = if self.settings.advanced {
            "高度なモードを有効にしました。危険な操作が表示されます。"
        } else {
            "高度なモードを無効にしました。"
        };
        self.set_status(StatusKind::Info, message);
    }

    pub fn load_slot(&mut self, path: &Path) {
        match SaveDocument::load(path) {
            Ok(document) => {
                self.settings.last_slot = document.slot_name();
                self.settings.save_dir = display_path(&self.save_dir);
                save_settings(&self.settings);
                if document.crc_warning.is_empty() {
                    self.set_status(
                        StatusKind::Success,
                        format!("{} を読み込みました。", document.slot_name()),
                    );
                } else {
                    self.set_status(
                        StatusKind::Error,
                        format!(
                            "{} を読み込みました。{}。保存すると CRC を再計算します。",
                            document.slot_name(),
                            document.crc_warning
                        ),
                    );
                }
                self.doc = Some(document);
                self.modal = None;
                self.pending = None;
                self.persona = None;
                self.picker = None;
                self.sync_all_drafts();
                if self.page == Page::Settings {
                    self.page = Page::Basic;
                }
            }
            Err(error) => {
                self.set_status(StatusKind::Error, format!("読み込み失敗: {error}"));
            }
        }
    }

    pub fn refresh_slots(&mut self) {
        let current = self.doc.as_ref().map(|doc| doc.path.clone());
        self.slots = scan_slots(&self.save_dir);
        if let Some(path) = current {
            if !self.slots.iter().any(|slot| slot.path == path) {
                self.doc = None;
                self.sync_all_drafts();
                self.set_status(
                    StatusKind::Error,
                    "読み込んでいたセーブが現在のフォルダにありません。保護のため閉じました。",
                );
            }
        }
    }

    pub fn refresh_backups(&mut self) {
        self.backups = if self.save_dir.as_os_str().is_empty() {
            Vec::new()
        } else {
            BackupManager::new(&self.save_dir).list()
        };
    }

    pub fn show_message(&mut self, title: impl Into<String>, message: impl Into<String>) {
        self.modal = Some(Modal::Message(title.into(), message.into()));
        self.pending = None;
    }

    pub fn request_confirm(
        &mut self,
        action: Action,
        title: impl Into<String>,
        message: impl Into<String>,
    ) {
        self.pending = Some(action);
        self.confirm_title = title.into();
        self.confirm_message = message.into();
        self.modal = Some(Modal::Confirm);
    }

    pub fn confirm_ok(&mut self) {
        let action = self.pending.take();
        self.modal = None;
        if let Some(action) = action {
            self.run_action(action);
        }
    }

    pub fn confirm_cancel(&mut self) {
        self.modal = None;
        self.pending = None;
        self.persona = None;
        self.picker = None;
    }

    pub fn close_modal(&mut self) {
        if matches!(self.modal, Some(Modal::Picker)) {
            self.picker = None;
            self.modal = Some(Modal::Persona);
            return;
        }
        self.modal = None;
        self.pending = None;
        self.persona = None;
        self.picker = None;
    }

    pub fn run_action(&mut self, action: Action) {
        match action {
            Action::Save => self.save_document(),
            Action::Revert => {
                if let Some(doc) = &mut self.doc {
                    doc.revert();
                    self.sync_all_drafts();
                    self.set_status(StatusKind::Info, "変更を元に戻しました。");
                }
            }
            Action::LoadSlot(path) => self.load_slot(&path),
            Action::ChangeFolder(path) => self.set_save_dir(path),
            Action::AutoDetect => {
                if let Some(path) = candidate_save_dirs().first().cloned() {
                    self.set_save_dir(path);
                } else {
                    self.show_message("自動検出", "セーブフォルダを自動検出できませんでした。");
                }
            }
            Action::RestoreBackup(index) => self.restore_backup(index),
            Action::CreateBackup => self.create_backup(),
            Action::MaxMoney => {
                if let Some(doc) = &mut self.doc {
                    doc.set_money(MONEY_MAX);
                    self.basic.money.set(MONEY_MAX.to_string());
                    self.set_status(StatusKind::Info, "所持金を MAX にしました（未保存）。");
                }
            }
            Action::MaxSocial => {
                if let Some(doc) = &mut self.doc {
                    doc.max_social();
                    self.sync_social_drafts();
                    self.set_status(
                        StatusKind::Info,
                        "ソーシャルステータスを MAX にしました（未保存）。",
                    );
                }
            }
            Action::MaxConfidants => {
                if let Some(doc) = &mut self.doc {
                    match doc.max_all_confidants() {
                        Ok(()) => {
                            self.sync_confidant_drafts();
                            self.set_status(
                                StatusKind::Info,
                                "コープをすべて MAX にしました（未保存）。",
                            );
                        }
                        Err(error) => self.set_status(StatusKind::Error, error.to_string()),
                    }
                }
            }
            Action::AllCompendium => {
                if let Some(doc) = &mut self.doc {
                    doc.all_compendium();
                    self.set_status(StatusKind::Info, "図鑑ビットを全登録にしました（未保存）。");
                }
            }
            Action::ClearCompendium => {
                if let Some(doc) = &mut self.doc {
                    doc.clear_compendium();
                    self.set_status(StatusKind::Info, "図鑑ビットを全解除しました（未保存）。");
                }
            }
            Action::MaxCategory(key) => {
                if let Some(doc) = &mut self.doc {
                    doc.max_items(&key);
                    self.sync_item_fields();
                    self.set_status(
                        StatusKind::Info,
                        "カテゴリをすべて所持/最大にしました（未保存）。",
                    );
                }
            }
            Action::ClearCategory(key) => {
                if let Some(doc) = &mut self.doc {
                    doc.clear_items(&key);
                    self.sync_item_fields();
                    self.set_status(StatusKind::Info, "カテゴリをすべて外しました（未保存）。");
                }
            }
        }
    }

    pub fn save_document(&mut self) {
        if self.doc.is_none() {
            self.show_message("保存", "セーブが読み込まれていません。");
            return;
        }
        if self.persona.is_some() {
            self.set_status(
                StatusKind::Error,
                "ペルソナ編集を「適用」または「キャンセル」してから保存してください。",
            );
            return;
        }
        if let Err(message) = self.commit_all_drafts() {
            self.set_status(StatusKind::Error, message);
            return;
        }
        let Some(doc) = &self.doc else {
            return;
        };
        if !doc.dirty {
            self.set_status(StatusKind::Info, "保存する変更はありません。");
            return;
        }
        if is_game_running() && !(self.advanced() && self.settings.allow_save_while_running) {
            self.show_message(
                "ゲーム起動中",
                "Persona 5 Royal が起動しています。セーブ破損と Steam Cloud の競合を避けるため、ゲームを終了してから保存してください。",
            );
            return;
        }
        let root = clean_path(
            &std::fs::canonicalize(&self.save_dir).unwrap_or_else(|_| self.save_dir.clone()),
        );
        let target = doc.path.clone();
        if !clean_path(&target).starts_with(&root) {
            self.show_message(
                "保存エラー",
                "読み込んだセーブが現在のセーブフォルダの外にあります。安全のため保存を中止しました。",
            );
            return;
        }
        let slot = doc.slot_name();
        if self.settings.auto_backup {
            if let Err(error) = BackupManager::new(&self.save_dir).create(
                &target,
                &slot,
                "保存前の自動バックアップ",
            ) {
                self.show_message(
                    "バックアップ失敗",
                    format!("バックアップを作成できないため保存を中止しました。\n{error}"),
                );
                return;
            }
        }
        let packed = {
            let Some(doc) = self.doc.as_mut() else {
                return;
            };
            match doc.to_bytes() {
                Ok(packed) => packed,
                Err(error) => {
                    self.show_message("保存失敗", error.to_string());
                    return;
                }
            }
        };
        if let Err(error) = SaveContainer::from_bytes(&packed, true) {
            self.show_message(
                "保存失敗",
                format!("生成したセーブの検証に失敗しました。\n{error}"),
            );
            return;
        }
        let Some(verified) = SaveContainer::from_bytes(&packed, true).ok() else {
            self.show_message("保存失敗", "生成したセーブを再読込できませんでした。");
            return;
        };
        let expected = self.doc.as_ref().map(|doc| doc.data.len()).unwrap_or(0);
        if !verified.is_pc_payload() || verified.data.len() != expected {
            self.show_message("保存失敗", "検証後のペイロードが一致しません。");
            return;
        }
        let temp = target.with_file_name("DATA.DAT.p5r-editor.tmp");
        let write_result = (|| -> std::io::Result<()> {
            std::fs::write(&temp, &packed)?;
            std::fs::rename(&temp, &target)
        })();
        if let Err(error) = write_result {
            let _ = std::fs::remove_file(&temp);
            self.show_message(
                "保存失敗",
                format!("ファイルに書き込めませんでした。\n{error}"),
            );
            return;
        }
        if let Some(doc) = &mut self.doc {
            doc.mark_saved(&packed);
        }
        self.set_status(StatusKind::Success, format!("{slot} を保存しました。"));
        self.sync_all_drafts();
        self.refresh_slots();
        self.refresh_backups();
    }

    pub fn create_backup(&mut self) {
        let Some(doc) = &self.doc else {
            self.show_message("バックアップ", "セーブが読み込まれていません。");
            return;
        };
        let path = doc.path.clone();
        let slot = doc.slot_name();
        match BackupManager::new(&self.save_dir).create(&path, &slot, "手動バックアップ") {
            Ok(record) => {
                self.set_status(
                    StatusKind::Success,
                    format!("バックアップを作成しました: {}", record.key),
                );
                self.refresh_backups();
            }
            Err(error) => self.show_message("バックアップ失敗", error.to_string()),
        }
    }

    pub fn restore_backup(&mut self, index: usize) {
        let Some(record) = self.backups.get(index).cloned() else {
            return;
        };
        if is_game_running() && !(self.advanced() && self.settings.allow_save_while_running) {
            self.show_message(
                "ゲーム起動中",
                "Persona 5 Royal が起動しています。ゲームを終了してから復元してください。",
            );
            return;
        }
        let target = self.save_dir.join(&record.slot).join("DATA.DAT");
        if let Err(error) = SaveDocument::load(&record.path) {
            self.show_message(
                "復元失敗",
                format!("バックアップを読み込めないため復元を中止しました。\n{error}"),
            );
            return;
        }
        match BackupManager::new(&self.save_dir).restore(&record, &target) {
            Ok(_) => {
                self.set_status(
                    StatusKind::Success,
                    format!("{} を復元しました。", record.slot),
                );
                self.load_slot(&target);
                self.refresh_backups();
            }
            Err(error) => self.show_message("復元失敗", error.to_string()),
        }
    }

    pub fn set_save_dir(&mut self, path: PathBuf) {
        self.save_dir = path;
        self.settings.save_dir = display_path(&self.save_dir);
        save_settings(&self.settings);
        self.slots = scan_slots(&self.save_dir);
        self.doc = None;
        self.modal = None;
        self.pending = None;
        self.persona = None;
        self.picker = None;
        self.sync_all_drafts();
        self.refresh_backups();
        let target = self
            .slots
            .iter()
            .find(|slot| slot.name == self.settings.last_slot && slot.valid)
            .or_else(|| {
                self.slots
                    .iter()
                    .filter(|slot| slot.valid)
                    .max_by_key(|slot| slot.mtime)
            })
            .map(|slot| slot.path.clone());
        if let Some(path) = target {
            self.load_slot(&path);
        } else {
            self.page = Page::Settings;
            self.set_status(
                StatusKind::Error,
                "選択したフォルダに有効なセーブがありません。",
            );
        }
    }

    pub fn request_load_slot(&mut self, path: PathBuf) {
        if self
            .doc
            .as_ref()
            .map(|doc| doc.path == path)
            .unwrap_or(false)
        {
            self.modal = None;
            return;
        }
        if self.doc.as_ref().map(|doc| doc.dirty).unwrap_or(false) {
            self.request_confirm(
                Action::LoadSlot(path),
                "未保存の変更",
                "未保存の変更があります。破棄して別のスロットを読み込みますか？",
            );
        } else {
            self.load_slot(&path);
            self.modal = None;
        }
    }

    pub fn request_change_folder(&mut self, path: PathBuf) {
        if path == self.save_dir {
            self.modal = None;
            return;
        }
        if self.doc.as_ref().map(|doc| doc.dirty).unwrap_or(false) {
            self.request_confirm(
                Action::ChangeFolder(path),
                "未保存の変更",
                "未保存の変更があります。破棄してセーブフォルダを変更しますか？",
            );
        } else {
            self.set_save_dir(path);
            self.modal = None;
        }
    }

    pub fn choose_folder(&mut self) {
        if let Some(folder) = rfd::FileDialog::new()
            .set_title("P5R の savedata フォルダを選択")
            .pick_folder()
        {
            self.request_change_folder(folder);
        }
    }

    pub fn open_backup_folder(&mut self) {
        open_in_explorer(&BackupManager::new(&self.save_dir).root);
    }

    pub fn open_data_folder(&mut self) {
        open_in_explorer(&local_app_dir());
    }

    pub fn toggle_auto_backup(&mut self) {
        self.settings.auto_backup = !self.settings.auto_backup;
        save_settings(&self.settings);
    }

    pub fn toggle_show_reserve(&mut self) {
        self.settings.show_reserve = !self.settings.show_reserve;
        save_settings(&self.settings);
    }

    pub fn toggle_allow_running(&mut self) {
        self.settings.allow_save_while_running = !self.settings.allow_save_while_running;
        save_settings(&self.settings);
    }

    // Drafts
    pub fn sync_all_drafts(&mut self) {
        self.sync_basic_drafts();
        self.sync_confidant_drafts();
        self.sync_member_drafts();
        self.sync_item_fields();
    }

    fn sync_basic_drafts(&mut self) {
        let Some(doc) = &self.doc else {
            return;
        };
        self.basic.name.set(doc.header.full_name());
        self.basic.money.set(doc.money().to_string());
        self.basic
            .playtime
            .set(input::format_playtime_input(doc.header.playtime_minutes()));
        self.basic
            .date
            .set(calendar::format_date_short(doc.header.day));
        let (_, payload_time) = doc.date();
        self.date_time = if calendar::is_valid_time(payload_time) {
            payload_time
        } else if calendar::is_valid_time(doc.header.time) {
            doc.header.time
        } else {
            4
        };
        self.sync_social_drafts();
    }

    fn sync_social_drafts(&mut self) {
        let Some(doc) = &self.doc else {
            return;
        };
        let stats = doc.social_stats();
        self.basic
            .social
            .resize_with(stats.len().max(5), || Field::new("0"));
        for (index, stat) in stats.iter().enumerate() {
            if let Some(field) = self.basic.social.get_mut(index) {
                field.set(stat.points.to_string());
            }
        }
    }

    fn sync_confidant_drafts(&mut self) {
        let Some(doc) = &self.doc else {
            return;
        };
        let states = doc.confidants();
        self.confidants
            .rank
            .resize_with(states.len(), || Field::new("0"));
        self.confidants
            .points
            .resize_with(states.len(), || Field::new("0"));
        for (index, state) in states.iter().enumerate() {
            if let Some(field) = self.confidants.rank.get_mut(index) {
                field.set(state.rank.to_string());
            }
            if let Some(field) = self.confidants.points.get_mut(index) {
                field.set(state.points.to_string());
            }
        }
    }

    fn sync_member_drafts(&mut self) {
        let Some(doc) = &self.doc else {
            return;
        };
        let characters = doc.characters();
        self.members
            .resize_with(characters.len(), MemberDrafts::new);
        for (index, character) in characters.iter().enumerate() {
            if let Some(fields) = self.members.get_mut(index) {
                fields.hp.set(character.hp.to_string());
                fields.sp.set(character.sp.to_string());
                fields.level.set(character.level.to_string());
                fields.xp.set(character.xp.to_string());
            }
        }
    }

    pub fn ensure_item_fields(&mut self) {
        let key = tables()
            .categories
            .get(self.category_index())
            .map(|category| category.key.clone())
            .unwrap_or_default();
        if key != self.item_fields_category {
            self.sync_item_fields();
        }
    }

    pub fn sync_item_fields(&mut self) {
        let Some(doc) = &self.doc else {
            self.item_fields.clear();
            self.item_fields_category.clear();
            return;
        };
        let Some(category) = tables().categories.get(self.category_index()) else {
            self.item_fields.clear();
            self.item_fields_category.clear();
            return;
        };
        self.item_fields_category = category.key.clone();
        self.item_fields
            .resize_with(category.count, || Field::new("0"));
        for index in 0..category.count {
            let value = doc.item_value(&category.key, index);
            self.item_fields[index].set(value.to_string());
        }
    }

    // Field validation
    pub fn money_value(&self) -> Option<u32> {
        input::parse_u32(self.basic.money.text()).filter(|value| *value <= MONEY_MAX)
    }

    pub fn playtime_value(&self) -> Option<u32> {
        input::parse_playtime_minutes(self.basic.playtime.text())
            .filter(|minutes| minutes.checked_mul(60 * 30).is_some())
    }

    pub fn date_value(&self) -> Option<u16> {
        calendar::parse_date(self.basic.date.text())
    }

    pub fn social_value(&self, index: usize) -> Option<u16> {
        input::parse_u16(self.basic.social.get(index)?.text()).filter(|value| *value <= 9999)
    }

    pub fn confidant_rank_value(&self, index: usize) -> Option<u16> {
        input::parse_u16(self.confidants.rank.get(index)?.text()).filter(|value| *value <= 10)
    }

    pub fn confidant_points_value(&self, index: usize) -> Option<u16> {
        input::parse_u16(self.confidants.points.get(index)?.text()).filter(|value| *value <= 999)
    }

    pub fn member_hp_value(&self, index: usize) -> Option<u32> {
        input::parse_u32(self.members.get(index)?.hp.text()).filter(|value| *value <= 9999)
    }

    pub fn member_sp_value(&self, index: usize) -> Option<u32> {
        input::parse_u32(self.members.get(index)?.sp.text()).filter(|value| *value <= 9999)
    }

    pub fn member_level_value(&self, index: usize) -> Option<u32> {
        input::parse_u32(self.members.get(index)?.level.text())
            .filter(|value| (1..=99).contains(value))
    }

    pub fn member_xp_value(&self, index: usize) -> Option<u32> {
        input::parse_u32(self.members.get(index)?.xp.text()).filter(|value| *value <= 2_000_000_000)
    }

    pub fn item_value_input(&self, index: usize) -> Option<u8> {
        input::parse_u8(self.item_fields.get(index)?.text()).filter(|value| *value <= 99)
    }

    pub fn persona_field_values(&self) -> Option<(u8, u32, [u8; 5])> {
        let persona = self.persona.as_ref()?;
        let level =
            input::parse_u8(persona.level.text()).filter(|value| (1..=99).contains(value))?;
        let xp = input::parse_u32(persona.xp.text()).filter(|value| *value <= 2_000_000_000)?;
        let mut stats = [0u8; 5];
        for (index, field) in persona.stats.iter().enumerate() {
            stats[index] = input::parse_u8(field.text()).filter(|value| *value <= 99)?;
        }
        Some((level, xp, stats))
    }

    // Live commits
    pub fn commit_name(&mut self) {
        let name = self.basic.name.text().trim().to_string();
        if name.is_empty() {
            return;
        }
        if let Some(doc) = &mut self.doc {
            doc.set_name(&name);
        }
    }

    pub fn commit_money(&mut self) {
        let Some(value) = self.money_value() else {
            return;
        };
        if let Some(doc) = &mut self.doc {
            doc.set_money(value);
        }
    }

    pub fn commit_playtime(&mut self) {
        let Some(value) = self.playtime_value() else {
            return;
        };
        if let Some(doc) = &mut self.doc {
            doc.set_playtime_minutes(value);
        }
    }

    pub fn commit_date(&mut self) {
        let Some(day) = self.date_value() else {
            return;
        };
        if let Some(doc) = &mut self.doc {
            if let Err(error) = doc.set_date(day, self.date_time) {
                self.set_status(StatusKind::Error, error.to_string());
            }
        }
    }

    pub fn set_difficulty(&mut self, value: u8) {
        if let Some(doc) = &mut self.doc {
            doc.set_difficulty(value);
        }
    }

    pub fn date_mismatch(&self) -> bool {
        self.doc
            .as_ref()
            .map(|doc| !doc.is_clear_data() && doc.date_mismatch())
            .unwrap_or(false)
    }

    pub fn select_time(&mut self, time: u8) {
        self.date_time = time;
        self.commit_date();
    }

    pub fn commit_social(&mut self, index: usize) {
        let Some(value) = self.social_value(index) else {
            return;
        };
        if let Some(doc) = &mut self.doc {
            doc.set_social_points(index, value);
        }
    }

    pub fn commit_confidant_rank(&mut self, index: usize) {
        let Some(rank) = self.confidant_rank_value(index) else {
            return;
        };
        let Some(points) = self
            .doc
            .as_ref()
            .and_then(|doc| doc.confidants().get(index).map(|state| state.points))
        else {
            return;
        };
        if let Some(doc) = &mut self.doc {
            if let Err(error) = doc.set_confidant(index, rank, points) {
                self.set_status(StatusKind::Error, error.to_string());
            }
        }
    }

    pub fn commit_confidant_points(&mut self, index: usize) {
        let Some(points) = self.confidant_points_value(index) else {
            return;
        };
        let Some(rank) = self
            .doc
            .as_ref()
            .and_then(|doc| doc.confidants().get(index).map(|state| state.rank))
        else {
            return;
        };
        if let Some(doc) = &mut self.doc {
            if let Err(error) = doc.set_confidant(index, rank, points) {
                self.set_status(StatusKind::Error, error.to_string());
            }
        }
    }

    pub fn commit_member(&mut self, index: usize) {
        let hp = self.member_hp_value(index);
        let sp = self.member_sp_value(index);
        let level = self.member_level_value(index);
        let xp = self.member_xp_value(index);
        let mut error = None;
        if let Some(doc) = &mut self.doc {
            if let Some(value) = hp {
                if let Err(err) = doc.set_character_hp(index, value) {
                    error = Some(err);
                }
            }
            if let Some(value) = sp {
                if let Err(err) = doc.set_character_sp(index, value) {
                    error = Some(err);
                }
            }
            if let Some(value) = level {
                if let Err(err) = doc.set_character_level(index, value) {
                    error = Some(err);
                }
            }
            if let Some(value) = xp {
                if let Err(err) = doc.set_character_xp(index, value) {
                    error = Some(err);
                }
            }
        }
        if let Some(error) = error {
            self.set_status(StatusKind::Error, error.to_string());
        }
    }

    pub fn commit_item(&mut self, index: usize) {
        let Some(category) = tables().categories.get(self.category_index()).cloned() else {
            return;
        };
        if category.read_only || category.kind == "owned" {
            return;
        }
        let Some(value) = self.item_value_input(index) else {
            return;
        };
        if let Some(doc) = &mut self.doc {
            doc.set_item_value(&category.key, index, value);
        }
    }

    pub fn toggle_item(&mut self, index: usize) {
        let Some(category) = tables().categories.get(self.category_index()).cloned() else {
            return;
        };
        if category.read_only || category.kind != "owned" {
            return;
        }
        let Some(doc) = &mut self.doc else {
            return;
        };
        let next = u8::from(doc.item_value(&category.key, index) == 0);
        doc.set_item_value(&category.key, index, next);
        if let Some(field) = self.item_fields.get_mut(index) {
            field.set(next.to_string());
        }
    }

    pub fn max_social_stat(&mut self, index: usize) {
        let Some(row) = tables().social_rows().get(index) else {
            return;
        };
        if let Some(&max) = row.thresholds.last() {
            if let Some(field) = self.basic.social.get_mut(index) {
                field.set(max.to_string());
            }
            self.commit_social(index);
        }
    }

    pub fn commit_persona_fields(&mut self) -> Result<(), String> {
        let Some((level, xp, stats)) = self.persona_field_values() else {
            return Err("ペルソナの数値が不正です。".into());
        };
        if let Some(persona) = &mut self.persona {
            persona.draft.level = level;
            persona.draft.xp = xp;
            persona.draft.stats = stats;
        }
        Ok(())
    }

    pub fn commit_persona_drafts(&mut self) {
        if let Ok(()) = self.commit_persona_fields() {}
    }

    // Save-time validation and application
    fn validate_all(&self) -> Result<(), String> {
        if self.basic.name.text().trim().is_empty() {
            return Err("名前を入力してください。".into());
        }
        if self.basic.name.text().trim().len() > crate::gamedata::NAME_LENGTH {
            return Err(format!(
                "名前は {} バイト以内で入力してください。",
                crate::gamedata::NAME_LENGTH
            ));
        }
        if self.money_value().is_none() {
            return Err("所持金は 0〜9,999,999 の数値で入力してください。".into());
        }
        if self.playtime_value().is_none() {
            return Err("プレイ時間は「87:09」または分単位の数値で入力してください。".into());
        }
        if self.date_value().is_none() {
            return Err("日付は「12/24」の形式で入力してください。".into());
        }
        if !calendar::is_valid_time(self.date_time) {
            return Err("時間帯が不正です。".into());
        }
        for index in 0..self.basic.social.len() {
            if self.social_value(index).is_none() {
                let name = tables()
                    .social_rows()
                    .get(index)
                    .map(|row| row.name.as_str())
                    .unwrap_or("ソーシャル");
                return Err(format!("{name} のポイントが不正です。"));
            }
        }
        let confidant_count = self.confidants.rank.len().min(self.confidants.points.len());
        for index in 0..confidant_count {
            if self.confidant_rank_value(index).is_none() {
                return Err("コープのランクは 0〜10 で入力してください。".into());
            }
            if self.confidant_points_value(index).is_none() {
                return Err("コープのポイントは 0〜999 で入力してください。".into());
            }
        }
        for index in 0..self.members.len() {
            if self.member_hp_value(index).is_none() || self.member_sp_value(index).is_none() {
                return Err("HP / SP は 0〜9999 で入力してください。".into());
            }
            if self.member_level_value(index).is_none() {
                return Err("レベルは 1〜99 で入力してください。".into());
            }
            if self.member_xp_value(index).is_none() {
                return Err("EXP が不正です。".into());
            }
        }
        if let Some(category) = tables().categories.get(self.category_index()) {
            if !category.read_only && category.kind != "owned" {
                for index in 0..category.count {
                    if self.item_value_input(index).is_none() {
                        return Err(format!(
                            "「{}」の個数は 0〜99 で入力してください。",
                            category.label
                        ));
                    }
                }
            }
        }
        Ok(())
    }

    fn apply_all(&mut self) -> Result<(), String> {
        let name = self.basic.name.text().trim().to_string();
        let money = self.money_value().ok_or("所持金が不正です。")?;
        let playtime = self.playtime_value().ok_or("プレイ時間が不正です。")?;
        let day = self.date_value().ok_or("日付が不正です。")?;
        let social: Vec<u16> = (0..self.basic.social.len())
            .filter_map(|index| self.social_value(index))
            .collect();
        let mut confidants = Vec::new();
        for index in 0..self.confidants.rank.len().min(self.confidants.points.len()) {
            if let (Some(rank), Some(points)) = (
                self.confidant_rank_value(index),
                self.confidant_points_value(index),
            ) {
                confidants.push((index, rank, points));
            }
        }
        let mut members = Vec::new();
        for index in 0..self.members.len() {
            members.push((
                index,
                self.member_hp_value(index),
                self.member_sp_value(index),
                self.member_level_value(index),
                self.member_xp_value(index),
            ));
        }
        let category = tables().categories.get(self.category_index()).cloned();
        let item_values: Vec<u8> = self
            .item_fields
            .iter()
            .map(|field| input::parse_u8(field.text()).unwrap_or(0))
            .collect();
        let Some(doc) = &mut self.doc else {
            return Err("セーブが読み込まれていません。".into());
        };
        if !doc.header.full_name().eq(&name) {
            doc.set_name(&name);
        }
        if doc.money() != money {
            doc.set_money(money);
        }
        if doc.header.playtime_minutes() != playtime {
            doc.set_playtime_minutes(playtime);
        }
        if !doc.is_clear_data()
            && (doc.header.day != day
                || doc.header.time != self.date_time
                || doc.date() != (day, self.date_time))
        {
            doc.set_date(day, self.date_time)
                .map_err(|error| error.to_string())?;
        }
        for (index, value) in social.into_iter().enumerate() {
            if doc
                .social_stats()
                .get(index)
                .map(|stat| stat.points != value)
                .unwrap_or(true)
            {
                doc.set_social_points(index, value);
            }
        }
        for (index, rank, points) in confidants {
            if let Some(state) = doc.confidants().get(index) {
                if state.rank != rank || state.points != points {
                    doc.set_confidant(index, rank, points)
                        .map_err(|error| error.to_string())?;
                }
            }
        }
        for (index, hp, sp, level, xp) in members {
            if let Some(character) = doc.characters().get(index) {
                if let Some(value) = hp {
                    if character.hp != value {
                        doc.set_character_hp(index, value)
                            .map_err(|error| error.to_string())?;
                    }
                }
                if let Some(value) = sp {
                    if character.sp != value {
                        doc.set_character_sp(index, value)
                            .map_err(|error| error.to_string())?;
                    }
                }
                if let Some(value) = level {
                    if character.level != value {
                        doc.set_character_level(index, value)
                            .map_err(|error| error.to_string())?;
                    }
                }
                if let Some(value) = xp {
                    if character.xp != value {
                        doc.set_character_xp(index, value)
                            .map_err(|error| error.to_string())?;
                    }
                }
            }
        }
        if let Some(category) = category {
            if !category.read_only && category.kind != "owned" {
                for (index, value) in item_values.into_iter().enumerate().take(category.count) {
                    if doc.item_value(&category.key, index) != value {
                        doc.set_item_value(&category.key, index, value);
                    }
                }
            }
        }
        Ok(())
    }

    pub fn commit_all_drafts(&mut self) -> Result<(), String> {
        self.validate_all()?;
        self.apply_all()?;
        Ok(())
    }

    // Personas
    pub fn open_persona(&mut self, character: usize, slot: usize) {
        let Some(doc) = &self.doc else {
            return;
        };
        let Some(definition) = tables().characters.get(character) else {
            return;
        };
        let Ok(view) = doc.persona_view(character, slot) else {
            return;
        };
        let draft = PersonaEdit {
            id: view.id,
            level: view.level.max(1),
            xp: view.xp,
            trait_id: view.trait_id,
            stats: if view.empty { [5; 5] } else { view.stats },
            skills: view.skills,
            clear: false,
        };
        self.selected_character = character;
        let level = Field::new(draft.level.to_string());
        let xp = Field::new(draft.xp.to_string());
        let stats = std::array::from_fn(|index| Field::new(draft.stats[index].to_string()));
        self.persona = Some(PersonaEditor {
            character,
            slot,
            draft,
            id_locked: !definition.joker,
            can_clear: definition.joker && slot != 0 && !view.empty,
            level,
            xp,
            stats,
        });
        self.modal = Some(Modal::Persona);
    }

    pub fn mark_persona_clear(&mut self) {
        if let Some(persona) = &mut self.persona {
            if persona.can_clear && self.settings.advanced {
                persona.draft.clear = !persona.draft.clear;
            }
        }
    }

    pub fn open_picker(&mut self, kind: PickerKind, target: PickerTarget) {
        let mut search = TextEdit::new("");
        search.focused = true;
        self.picker = Some(Picker {
            kind,
            target,
            page: 0,
            search,
        });
        self.modal = Some(Modal::Picker);
    }

    pub fn picker_rows(&self) -> Vec<(u16, String)> {
        let Some(picker) = &self.picker else {
            return Vec::new();
        };
        let query = picker.search.text.to_lowercase();
        let matches = |id: u16, name: &str| {
            query.is_empty()
                || name.to_lowercase().contains(&query)
                || format!("{id}").contains(&query)
        };
        match picker.kind {
            PickerKind::Persona => tables()
                .personas()
                .iter()
                .filter(|row| row.id != 0)
                .filter(|row| !is_reserve_name(&row.name) && matches(row.id, &row.name))
                .map(|row| (row.id, row.name.clone()))
                .collect(),
            PickerKind::Skill => {
                let mut rows = Vec::new();
                if matches(0, "なし") {
                    rows.push((0, "なし".to_string()));
                }
                rows.extend(
                    tables()
                        .skills()
                        .iter()
                        .filter(|row| row.id != 0)
                        .filter(|row| !is_reserve_name(&row.name) && matches(row.id, &row.name))
                        .map(|row| (row.id, row.name.clone())),
                );
                rows
            }
            PickerKind::Trait => {
                let mut rows = Vec::new();
                if matches(0, "なし") {
                    rows.push((0, "なし".to_string()));
                }
                rows.extend(
                    tables()
                        .traits()
                        .iter()
                        .filter(|row| row.id != 0)
                        .filter(|row| !is_reserve_name(&row.name) && matches(row.id, &row.name))
                        .map(|row| (row.id, row.name.clone())),
                );
                rows
            }
        }
    }

    pub fn picker_select(&mut self, id: u16) {
        let target = self.picker.as_ref().map(|picker| picker.target);
        if let (Some(editor), Some(target)) = (&mut self.persona, target) {
            match target {
                PickerTarget::PersonaId => {
                    editor.draft.id = id;
                    editor.draft.clear = false;
                }
                PickerTarget::Trait => editor.draft.trait_id = id,
                PickerTarget::Skill(slot) => {
                    if slot < 8 {
                        editor.draft.skills[slot] = id;
                    }
                }
            }
        }
        self.picker = None;
        self.modal = Some(Modal::Persona);
    }

    pub fn apply_persona(&mut self) {
        if let Err(message) = self.commit_persona_fields() {
            self.set_status(StatusKind::Error, message);
            return;
        }
        let Some(editor) = self.persona.take() else {
            return;
        };
        if editor.draft.clear {
            if editor.can_clear {
                let mut error = None;
                if let Some(doc) = &mut self.doc {
                    if let Err(err) = doc.clear_persona(editor.character, editor.slot) {
                        error = Some(err);
                    }
                }
                if let Some(error) = error {
                    self.set_status(StatusKind::Error, error.to_string());
                    self.persona = Some(editor);
                    return;
                }
                self.set_status(
                    StatusKind::Info,
                    "ペルソナスロットを空にしました（未保存）。",
                );
            }
        } else if editor.draft.id == 0 {
            self.set_status(StatusKind::Error, "ペルソナを選択してください。");
            self.persona = Some(editor);
            return;
        } else if let Some(doc) = &mut self.doc {
            match doc.apply_persona(editor.character, editor.slot, &editor.draft) {
                Ok(()) => {
                    self.set_status(StatusKind::Info, "ペルソナを更新しました（未保存）。");
                }
                Err(error) => {
                    self.set_status(StatusKind::Error, error.to_string());
                    self.persona = Some(editor);
                    return;
                }
            }
        }
        self.sync_member_drafts();
        self.modal = None;
    }

    // Items / compendium
    pub fn category_index(&self) -> usize {
        self.selected_category
            .min(tables().categories.len().saturating_sub(1))
    }

    pub fn set_category(&mut self, index: usize) {
        self.selected_category = index;
        self.item_page = 0;
        self.sync_item_fields();
    }

    pub fn item_indices(&self) -> Vec<usize> {
        let Some(category) = tables().categories.get(self.category_index()) else {
            return Vec::new();
        };
        let query = self.item_search.text.to_lowercase();
        (0..category.count)
            .filter(|&index| {
                let name = tables().item_name(&category.key, index);
                if !self.settings.show_reserve && is_reserve_name(&name) {
                    return false;
                }
                if !query.is_empty() && !name.to_lowercase().contains(&query) {
                    return false;
                }
                if self.owned_only
                    && self
                        .doc
                        .as_ref()
                        .map(|doc| doc.item_value(&category.key, index) == 0)
                        .unwrap_or(true)
                {
                    return false;
                }
                true
            })
            .collect()
    }

    pub fn max_category(&mut self) {
        let Some(category) = tables().categories.get(self.category_index()).cloned() else {
            return;
        };
        self.request_confirm(
            Action::MaxCategory(category.key.clone()),
            "一括所持 / 最大",
            format!(
                "「{}」の全 {} 件を所持/最大にします。実行しますか？",
                category.label, category.count
            ),
        );
    }

    pub fn clear_category(&mut self) {
        let Some(category) = tables().categories.get(self.category_index()).cloned() else {
            return;
        };
        self.request_confirm(
            Action::ClearCategory(category.key.clone()),
            "一括解除",
            format!(
                "「{}」の全 {} 件を 0 にします。実行しますか？",
                category.label, category.count
            ),
        );
    }

    pub fn compendium_ids(&self) -> Vec<u16> {
        let query = self.compendium_search.text.to_lowercase();
        (1..=232)
            .filter(|&id| {
                query.is_empty() || tables().persona_name(id).to_lowercase().contains(&query)
            })
            .collect()
    }

    pub fn set_compendium(&mut self, id: u16, registered: bool) {
        if let Some(doc) = &mut self.doc {
            doc.set_compendium(id, registered);
        }
    }

    pub fn confidant_filter(&self) -> String {
        self.confidant_search.text.trim().to_lowercase()
    }
}

pub fn is_reserve_name(name: &str) -> bool {
    matches!(
        name.trim(),
        "" | "未使用"
            | "???"
            | "？？？"
            | "----------"
            | "－－－－－－"
            | "BLANK"
            | "ブランク"
            | "ブランク（使用不可）"
            | "BLANK (Unusable)"
            | "使用不可"
            | "リザーブ"
            | "RESERVE"
            | "予備"
            | "Unused"
    )
}

pub fn scan_slots(save_dir: &Path) -> Vec<SlotSummary> {
    slot_files(save_dir)
        .into_iter()
        .map(|slot| {
            let mtime = std::fs::metadata(&slot.path)
                .and_then(|meta| meta.modified())
                .ok()
                .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|duration| duration.as_secs())
                .unwrap_or(0);
            let mut valid = false;
            let mut label = if slot.exists {
                format!("{}  |  読み込み失敗", slot.name)
            } else {
                format!("{}  |  空", slot.name)
            };
            if slot.exists {
                if let Ok(document) = SaveDocument::load(&slot.path) {
                    valid = true;
                    label = format!(
                        "{}  |  {}  |  {}  |  Lv {}",
                        slot.name,
                        document.header.full_name(),
                        calendar::format_date(document.header.day),
                        document.header.level
                    );
                }
            }
            SlotSummary {
                name: slot.name,
                path: slot.path,
                valid,
                label,
                mtime,
            }
        })
        .collect()
}

pub fn view(s: &Editor, cx: &mut Cx<Editor>) -> El {
    let metrics = Metrics::from_size(cx.layout_size("app-root"));
    let t = ui::theme();
    let sidebar = sidebar(s, &metrics);
    let content = col()
        .grow(1.0)
        .h_full()
        .min_h(0.0)
        .child(topbar(s, &metrics))
        .child(
            col()
                .grow(1.0)
                .min_h(0.0)
                .scroll(ScrollSpec {
                    stick_bottom: false,
                })
                .child(views::page(s, &metrics)),
        )
        .child(statusbar(s, &metrics));
    let mut root = row()
        .key("app-root")
        .w_full()
        .h_full()
        .min_h(0.0)
        .fill(t.bg)
        .child(sidebar)
        .child(content);
    if let Some(modal) = views::modal(s, &metrics) {
        root = root.child(modal);
    }
    root
}

fn sidebar(s: &Editor, m: &Metrics) -> El {
    let t = ui::theme();
    let mut nav = col()
        .w(m.sidebar())
        .h_full()
        .padding(m.x(12.0))
        .gap(m.x(2.0))
        .fill(t.card);
    nav = nav.child(
        row()
            .h(m.x(34.0))
            .gap(m.x(6.0))
            .center_y()
            .child(
                text("P5R")
                    .font(Metrics::font(m, rune::tok::type_::TITLE))
                    .color(ui::accent()),
            )
            .child(
                text("SAVE EDITOR")
                    .font(ui::small_style(m))
                    .color(t.text_secondary),
            ),
    );
    nav = nav.child(el_gap(m, 4.0));
    for page in Page::ALL {
        let selected = s.page == page;
        nav = nav.child(
            row()
                .h(m.x(29.0))
                .px(m.x(8.0))
                .gap(m.x(6.0))
                .center_y()
                .radius(m.x(5.0))
                .fill(if selected {
                    ui::accent().alpha(0.14)
                } else {
                    Color::TRANSPARENT
                })
                .hover(t.hover)
                .cursor(rune::Cursor::Pointer)
                .on_press(move |s: &mut Editor| s.set_page(page))
                .child(
                    text(page.label())
                        .font(if selected {
                            Metrics::font(m, rune::tok::type_::TITLE)
                        } else {
                            m.body()
                        })
                        .color(if selected {
                            ui::accent()
                        } else {
                            t.text_secondary
                        }),
                ),
        );
    }
    nav = nav.child(spacer().grow(1.0));
    let advanced = s.advanced();
    nav = nav.child(
        row()
            .w_full()
            .h(m.x(28.0))
            .px(m.x(8.0))
            .gap(m.x(7.0))
            .center_y()
            .radius(m.x(5.0))
            .hover(t.hover)
            .cursor(rune::Cursor::Pointer)
            .on_press(|s: &mut Editor| s.toggle_advanced())
            .child(ui::checkbox(m, advanced))
            .child(text("高度なモード").font(m.label()).color(if advanced {
                ui::warning()
            } else {
                t.text_secondary
            })),
    );
    nav = nav.child(
        row().w_full().px(m.x(8.0)).center_y().child(
            text(format!("v{APP_VERSION}"))
                .font(ui::small_style(m))
                .color(t.text_secondary),
        ),
    );
    nav.border(1.0, t.card_border)
}

fn topbar(s: &Editor, m: &Metrics) -> El {
    let t = ui::theme();
    let slot_label = s
        .doc
        .as_ref()
        .map(|doc| {
            format!(
                "{}  |  {}  |  {}",
                doc.slot_name(),
                doc.header.full_name(),
                calendar::format_date(doc.header.day)
            )
        })
        .unwrap_or_else(|| "セーブ未読込".into());
    let mut bar = row()
        .h(m.x(46.0))
        .w_full()
        .px(m.content_pad())
        .gap(m.x(8.0))
        .center_y()
        .fill(t.card)
        .border(1.0, t.card_border)
        .child(
            row()
                .h(m.x(28.0))
                .px(m.x(9.0))
                .gap(m.x(8.0))
                .center_y()
                .radius(m.x(5.0))
                .fill(t.control_track)
                .hover(t.hover)
                .cursor(rune::Cursor::Pointer)
                .on_press(|s: &mut Editor| s.modal = Some(Modal::Slots))
                .child(
                    text(slot_label)
                        .font(m.body())
                        .color(t.text)
                        .max_w(m.x(420.0))
                        .truncate(),
                )
                .child(
                    text("変更")
                        .font(ui::small_style(m))
                        .color(t.text_secondary),
                ),
        )
        .child(spacer());
    if s.doc.is_some() {
        if s.doc.as_ref().map(|doc| doc.dirty).unwrap_or(false) {
            bar = bar.child(ui::tag(m, "未保存", ui::warning()));
        } else {
            bar = bar.child(
                text("保存済み")
                    .font(ui::small_style(m))
                    .color(t.text_secondary),
            );
        }
        bar = bar
            .child(
                ui::primary(m, "保存")
                    .key("save")
                    .on_press(|s: &mut Editor| s.save_document()),
            )
            .child(ui::ghost(m, "元に戻す").on_press(|s: &mut Editor| {
                if s.doc.as_ref().map(|doc| doc.dirty).unwrap_or(false) {
                    s.request_confirm(
                        Action::Revert,
                        "変更を元に戻す",
                        "未保存の変更を破棄して、読み込み時の状態に戻しますか？",
                    );
                }
            }))
            .child(ui::ghost(m, "バックアップ").on_press(|s: &mut Editor| {
                s.request_confirm(
                    Action::CreateBackup,
                    "バックアップ",
                    "現在のセーブのバックアップを今すぐ作成しますか？",
                );
            }));
    } else {
        bar = bar.child(
            ui::primary(m, "スロットを開く")
                .on_press(|s: &mut Editor| s.modal = Some(Modal::Slots)),
        );
    }
    bar
}

fn statusbar(s: &Editor, m: &Metrics) -> El {
    let t = ui::theme();
    let color = match s.status_kind {
        StatusKind::Info => t.text_secondary,
        StatusKind::Error => ui::danger(),
        StatusKind::Success => ui::success(),
    };
    let mut bar = row()
        .w_full()
        .h(m.x(24.0))
        .px(m.content_pad())
        .gap(m.x(10.0))
        .center_y()
        .fill(t.card)
        .border(1.0, t.card_border)
        .child(
            text(if s.status.is_empty() {
                "準備完了"
            } else {
                s.status.as_str()
            })
            .font(ui::small_style(m))
            .color(color)
            .truncate()
            .grow(1.0)
            .min_w(0.0),
        );
    if s.advanced() {
        bar = bar.child(ui::tag(m, "高度", ui::warning()));
    }
    bar.child(
        text(format!("v{APP_VERSION}"))
            .font(ui::small_style(m))
            .color(t.text_secondary),
    )
}

fn el_gap(m: &Metrics, height: f32) -> El {
    rune::el().h(m.x(height))
}
