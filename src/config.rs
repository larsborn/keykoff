use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunConfig {
    pub name: String,
    #[serde(default)]
    pub caption: String,
    pub executable: String,
    pub parameters: String,
    pub working_directory: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunGroup {
    pub name: String,
    #[serde(default)]
    pub caption: String,
    #[serde(default)]
    pub members: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Entry {
    Program(RunConfig),
    Group(RunGroup),
}

impl<'de> serde::Deserialize<'de> for Entry {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let mut value = serde_json::Value::deserialize(deserializer)?;
        if let Some(obj) = value.as_object_mut() {
            if !obj.contains_key("kind") {
                obj.insert("kind".into(), serde_json::Value::String("program".into()));
            }
        }
        // Re-deserialize through a private helper enum that uses the standard tagged-enum derivation.
        #[derive(serde::Deserialize)]
        #[serde(tag = "kind", rename_all = "lowercase")]
        enum Helper {
            Program(RunConfig),
            Group(RunGroup),
        }
        let h: Helper = serde_json::from_value(value).map_err(serde::de::Error::custom)?;
        Ok(match h {
            Helper::Program(p) => Entry::Program(p),
            Helper::Group(g) => Entry::Group(g),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub entries: Vec<RunConfig>,
    #[serde(default = "default_overlay_x")]
    pub overlay_x: f32,
    #[serde(default = "default_overlay_y")]
    pub overlay_y: f32,
    #[serde(default = "default_overlay_width")]
    pub overlay_width: f32,
    #[serde(default = "default_hotkey_key")]
    pub hotkey_key: String,
    #[serde(default = "default_true")]
    pub hotkey_alt: bool,
    #[serde(default)]
    pub hotkey_ctrl: bool,
}

fn default_overlay_x() -> f32 {
    100.0
}

fn default_overlay_y() -> f32 {
    100.0
}

fn default_overlay_width() -> f32 {
    400.0
}

fn default_hotkey_key() -> String {
    "F10".to_string()
}

fn default_true() -> bool {
    true
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            overlay_x: default_overlay_x(),
            overlay_y: default_overlay_y(),
            overlay_width: default_overlay_width(),
            hotkey_key: default_hotkey_key(),
            hotkey_alt: true,
            hotkey_ctrl: false,
        }
    }
}

pub fn config_path() -> PathBuf {
    let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push("keykoff");
    path.push("config.json");
    path
}

pub fn load_config() -> AppConfig {
    let path = config_path();
    if !path.exists() {
        return AppConfig::default();
    }
    match std::fs::read_to_string(&path) {
        Ok(contents) => serde_json::from_str(&contents).unwrap_or_default(),
        Err(_) => AppConfig::default(),
    }
}

pub fn save_config(config: &AppConfig) -> Result<(), Box<dyn std::error::Error>> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(config)?;
    std::fs::write(&path, json)?;
    Ok(())
}

pub fn entry_name(entry: &Entry) -> &str {
    match entry {
        Entry::Program(p) => &p.name,
        Entry::Group(g) => &g.name,
    }
}

pub fn find_by_name(entries: &[Entry], name: &str) -> Option<usize> {
    entries.iter().position(|e| entry_name(e) == name)
}

pub fn cascade_rename(entries: &mut [Entry], old_name: &str, new_name: &str) {
    if old_name == new_name {
        return;
    }
    for entry in entries.iter_mut() {
        if let Entry::Group(g) = entry {
            for m in g.members.iter_mut() {
                if m == old_name {
                    *m = new_name.to_string();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_entry_without_kind_loads_as_program() {
        let json = r#"{"name":"foo","caption":"","executable":"x.exe","parameters":"","working_directory":""}"#;
        let entry: Entry = serde_json::from_str(json).unwrap();
        match entry {
            Entry::Program(p) => assert_eq!(p.name, "foo"),
            _ => panic!("expected Program, got {:?}", entry),
        }
    }

    #[test]
    fn entry_with_kind_program_loads_as_program() {
        let json = r#"{"kind":"program","name":"foo","caption":"","executable":"x.exe","parameters":"","working_directory":""}"#;
        let entry: Entry = serde_json::from_str(json).unwrap();
        assert!(matches!(entry, Entry::Program(_)));
    }

    #[test]
    fn entry_with_kind_group_loads_as_group() {
        let json = r#"{"kind":"group","name":"g","caption":"","members":["a","b"]}"#;
        let entry: Entry = serde_json::from_str(json).unwrap();
        match entry {
            Entry::Group(g) => {
                assert_eq!(g.name, "g");
                assert_eq!(g.members, vec!["a".to_string(), "b".to_string()]);
            }
            _ => panic!("expected Group, got {:?}", entry),
        }
    }

    #[test]
    fn group_round_trip() {
        let g = Entry::Group(RunGroup {
            name: "g".into(),
            caption: "c".into(),
            members: vec!["a".into()],
        });
        let s = serde_json::to_string(&g).unwrap();
        let parsed: Entry = serde_json::from_str(&s).unwrap();
        assert!(matches!(parsed, Entry::Group(_)));
        assert!(s.contains(r#""kind":"group""#));
    }

    #[test]
    fn program_round_trip_writes_kind() {
        let p = Entry::Program(RunConfig {
            name: "p".into(),
            caption: String::new(),
            executable: "p.exe".into(),
            parameters: String::new(),
            working_directory: String::new(),
        });
        let s = serde_json::to_string(&p).unwrap();
        assert!(s.contains(r#""kind":"program""#));
    }

    fn make_program(name: &str) -> Entry {
        Entry::Program(RunConfig {
            name: name.into(),
            caption: String::new(),
            executable: format!("{}.exe", name),
            parameters: String::new(),
            working_directory: String::new(),
        })
    }

    fn make_group(name: &str, members: &[&str]) -> Entry {
        Entry::Group(RunGroup {
            name: name.into(),
            caption: String::new(),
            members: members.iter().map(|s| s.to_string()).collect(),
        })
    }

    fn entry_name(e: &Entry) -> &str {
        super::entry_name(e)
    }

    #[test]
    fn find_by_name_returns_index_when_present() {
        let entries = vec![make_program("a"), make_group("g", &["a"]), make_program("b")];
        assert_eq!(find_by_name(&entries, "g"), Some(1));
        assert_eq!(find_by_name(&entries, "b"), Some(2));
    }

    #[test]
    fn find_by_name_returns_none_when_missing() {
        let entries = vec![make_program("a")];
        assert_eq!(find_by_name(&entries, "missing"), None);
    }

    #[test]
    fn cascade_rename_updates_group_members() {
        let mut entries = vec![
            make_program("a"),
            make_program("b"),
            make_group("g", &["a", "b"]),
        ];
        cascade_rename(&mut entries, "a", "a2");
        if let Entry::Group(g) = &entries[2] {
            assert_eq!(g.members, vec!["a2".to_string(), "b".to_string()]);
        } else {
            panic!("expected group at index 2");
        }
    }

    #[test]
    fn cascade_rename_no_op_when_name_not_referenced() {
        let mut entries = vec![make_program("a"), make_group("g", &["b"])];
        cascade_rename(&mut entries, "a", "a2");
        if let Entry::Group(g) = &entries[1] {
            assert_eq!(g.members, vec!["b".to_string()]);
        }
    }

    #[test]
    fn cascade_rename_updates_multiple_groups() {
        let mut entries = vec![
            make_program("a"),
            make_group("g1", &["a"]),
            make_group("g2", &["a", "x"]),
        ];
        cascade_rename(&mut entries, "a", "a2");
        if let Entry::Group(g) = &entries[1] { assert_eq!(g.members, vec!["a2".to_string()]); }
        if let Entry::Group(g) = &entries[2] { assert_eq!(g.members, vec!["a2".to_string(), "x".to_string()]); }
    }
}
