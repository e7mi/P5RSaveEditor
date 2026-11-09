//! Embedded name tables and static game definitions.

use std::collections::HashMap;
use std::sync::OnceLock;

use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct NameRow {
    pub id: u16,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub name_en: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ConfidantRow {
    pub save_id: u16,
    #[serde(default)]
    pub arcana: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub story_locked: bool,
}

#[derive(Clone, Debug, Deserialize)]
pub struct SocialRow {
    #[serde(default)]
    pub key: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub thresholds: Vec<u16>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct CharacterDef {
    pub key: String,
    pub name: String,
    pub base: usize,
    #[serde(default)]
    pub joker: bool,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ItemCategoryDef {
    pub key: String,
    pub label: String,
    pub base: usize,
    pub count: usize,
    pub kind: String,
    #[serde(default)]
    pub read_only: bool,
}

#[derive(Clone, Debug)]
pub struct ConfidantDef {
    pub save_ids: Vec<u16>,
    pub arcana: String,
    pub name: String,
    pub story_locked: bool,
}

#[derive(Deserialize)]
struct RawTables {
    #[serde(default)]
    personas: Vec<NameRow>,
    #[serde(default)]
    skills: Vec<NameRow>,
    #[serde(default)]
    traits: Vec<NameRow>,
    #[serde(default)]
    items: HashMap<String, Vec<NameRow>>,
    #[serde(default)]
    confidants: Vec<ConfidantRow>,
    #[serde(default)]
    social_stats: Vec<SocialRow>,
}

pub struct Tables {
    raw: RawTables,
    persona_names: HashMap<u16, String>,
    skill_names: HashMap<u16, String>,
    trait_names: HashMap<u16, String>,
    pub confidants: Vec<ConfidantDef>,
    pub characters: Vec<CharacterDef>,
    pub categories: Vec<ItemCategoryDef>,
}

impl Tables {
    fn load() -> Self {
        let raw: RawTables =
            serde_json::from_str(include_str!("../assets/gamedata.json")).expect("gamedata.json");
        let characters: Vec<CharacterDef> =
            serde_json::from_str(include_str!("../assets/characters.json"))
                .expect("characters.json");
        let categories: Vec<ItemCategoryDef> =
            serde_json::from_str(include_str!("../assets/categories.json"))
                .expect("categories.json");

        let names = |rows: &[NameRow]| {
            rows.iter()
                .filter(|row| row.id != 0)
                .map(|row| {
                    let name = if row.name.is_empty() {
                        row.name_en.clone()
                    } else {
                        row.name.clone()
                    };
                    (row.id, name)
                })
                .collect::<HashMap<_, _>>()
        };
        let persona_names = names(&raw.personas);
        let skill_names = names(&raw.skills);
        let trait_names = names(&raw.traits);

        let mut rows = raw.confidants.clone();
        rows.sort_by_key(|row| row.save_id);
        let mut confidants = Vec::new();
        let mut seen_faith = false;
        for row in rows {
            if row.save_id == 36 {
                continue;
            }
            if row.arcana == "信念" {
                if seen_faith {
                    continue;
                }
                seen_faith = true;
                confidants.push(ConfidantDef {
                    save_ids: vec![33, 36],
                    arcana: row.arcana,
                    name: if row.name.is_empty() {
                        "芳澤すみれ".to_string()
                    } else {
                        row.name
                    },
                    story_locked: row.story_locked,
                });
            } else {
                confidants.push(ConfidantDef {
                    save_ids: vec![row.save_id],
                    arcana: row.arcana,
                    name: row.name,
                    story_locked: row.story_locked,
                });
            }
        }
        let order = |id: u16| match id {
            1..=21 => id as i32 - 1,
            33 => 21,
            35 => 22,
            _ => 99,
        };
        confidants.sort_by_key(|def| order(def.save_ids[0]));

        Self {
            raw,
            persona_names,
            skill_names,
            trait_names,
            confidants,
            characters,
            categories,
        }
    }

    pub fn persona_name(&self, id: u16) -> String {
        if id == 0 {
            return "-".into();
        }
        self.persona_names
            .get(&id)
            .cloned()
            .unwrap_or_else(|| format!("ID {id}"))
    }

    pub fn skill_name(&self, id: u16) -> String {
        if id == 0 {
            return "なし".into();
        }
        self.skill_names
            .get(&id)
            .cloned()
            .unwrap_or_else(|| format!("ID {id}"))
    }

    pub fn trait_name(&self, id: u16) -> String {
        if id == 0 {
            return "なし".into();
        }
        self.trait_names
            .get(&id)
            .cloned()
            .unwrap_or_else(|| format!("ID {id}"))
    }

    pub fn item_name(&self, category: &str, index: usize) -> String {
        let prefix = item_prefix(category);
        let id = prefix + index as u16;
        let Some(rows) = self.raw.items.get(category) else {
            return format!("ID {id:04X}");
        };
        let Some(row) = rows.get(index) else {
            return format!("ID {id:04X}");
        };
        if !row.name.is_empty() {
            row.name.clone()
        } else if !row.name_en.is_empty() {
            row.name_en.clone()
        } else {
            format!("ID {id:04X}")
        }
    }

    pub fn social_rows(&self) -> &[SocialRow] {
        &self.raw.social_stats
    }

    pub fn personas(&self) -> &[NameRow] {
        &self.raw.personas
    }

    pub fn skills(&self) -> &[NameRow] {
        &self.raw.skills
    }

    pub fn traits(&self) -> &[NameRow] {
        &self.raw.traits
    }

    pub fn category(&self, key: &str) -> Option<&ItemCategoryDef> {
        self.categories.iter().find(|category| category.key == key)
    }
}

pub fn item_prefix(category: &str) -> u16 {
    match category {
        "melee" => 0x1000,
        "ranged" => 0x7000,
        "armor" => 0x5000,
        "accessory" => 0x3000,
        "consumable" => 0x2000,
        "skillcard" => 0x4000,
        "material" => 0x8000,
        "outfit" => 0xA000,
        "keyitem" => 0x9000,
        _ => 0,
    }
}

pub fn tables() -> &'static Tables {
    static TABLES: OnceLock<Tables> = OnceLock::new();
    TABLES.get_or_init(Tables::load)
}
