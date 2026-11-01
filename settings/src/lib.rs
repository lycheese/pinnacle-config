//! Settings shared by the config and `pctl`.
//!
//! `settings.toml` in the repo holds documented defaults. `local.toml` next to it (gitignored)
//! overrides any key; tables are merged key by key. Values are looked up with dotted paths,
//! e.g. `settings.str("apps.terminal")`.
//!
//! The repo folder is taken from `PINNACLE_CONFIG_REPO` if set, else the folder this crate was
//! built from (the config and pctl are always built from the repo).

use std::path::{Path, PathBuf};

use toml::Table;
pub use toml::Value;

/// Folder holding `settings.toml` and `local.toml`.
pub fn repo_dir() -> PathBuf {
    std::env::var_os("PINNACLE_CONFIG_REPO")
        .map(PathBuf::from)
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join(".."))
}

#[derive(Debug, Clone, Default)]
pub struct Settings {
    table: Table,
}

impl Settings {
    /// Loads and merges the settings files. Missing or broken files are reported on stderr and
    /// skipped, so a typo in `local.toml` never stops the config from starting.
    pub fn load() -> Self {
        let dir = repo_dir();
        let mut table = read_table(&dir.join("settings.toml")).unwrap_or_default();
        if let Some(local) = read_table(&dir.join("local.toml")) {
            merge(&mut table, local);
        }
        Self { table }
    }

    /// Settings from TOML text alone (no files), e.g. `settings.toml` in tests.
    pub fn parse(text: &str) -> Result<Self, toml::de::Error> {
        Ok(Self {
            table: text.parse()?,
        })
    }

    pub fn get(&self, path: &str) -> Option<&Value> {
        let mut parts = path.split('.');
        let mut value = self.table.get(parts.next()?)?;
        for part in parts {
            value = value.as_table()?.get(part)?;
        }
        Some(value)
    }

    /// A string setting; `default` if missing or not a string.
    pub fn str(&self, path: &str, default: &str) -> String {
        self.get(path)
            .and_then(Value::as_str)
            .unwrap_or(default)
            .to_string()
    }

    /// A string setting with a leading `~` expanded to the home folder.
    pub fn path(&self, path: &str, default: &str) -> PathBuf {
        PathBuf::from(expand_home(&self.str(path, default)))
    }

    pub fn bool(&self, path: &str, default: bool) -> bool {
        self.get(path).and_then(Value::as_bool).unwrap_or(default)
    }

    /// Every setting as `export PCFG_<SECTION>_<KEY>='value'` lines for `eval` in shell scripts.
    /// Arrays become `:`-separated lists (paths are `~`-expanded), like `$PATH`.
    pub fn to_env(&self) -> String {
        let mut out = String::new();
        let mut push = |name: &str, value: &Value| {
            let text = match value {
                Value::String(s) => expand_home(s),
                Value::Array(items) => items
                    .iter()
                    .map(|v| match v {
                        Value::String(s) => expand_home(s),
                        other => other.to_string(),
                    })
                    .collect::<Vec<_>>()
                    .join(":"),
                Value::Table(_) => return,
                other => other.to_string(),
            };
            let quoted = text.replace('\'', r"'\''");
            out.push_str(&format!("export PCFG_{name}='{quoted}'\n"));
        };
        for (section, value) in &self.table {
            match value {
                Value::Table(keys) => {
                    for (key, value) in keys {
                        push(&env_name(&format!("{section}_{key}")), value);
                    }
                }
                other => push(&env_name(section), other),
            }
        }
        out
    }
}

fn env_name(key: &str) -> String {
    key.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect()
}

fn read_table(path: &Path) -> Option<Table> {
    let text = std::fs::read_to_string(path).ok()?;
    match text.parse::<Table>() {
        Ok(table) => Some(table),
        Err(err) => {
            eprintln!("settings: ignoring {}: {err}", path.display());
            None
        }
    }
}

/// Merges `over` into `base`: nested tables merge key by key, everything else is replaced.
fn merge(base: &mut Table, over: Table) {
    for (key, value) in over {
        match (base.get_mut(&key), value) {
            (Some(Value::Table(base_table)), Value::Table(over_table)) => {
                merge(base_table, over_table);
            }
            (_, value) => {
                base.insert(key, value);
            }
        }
    }
}

pub fn expand_home(path: &str) -> String {
    match (path.strip_prefix('~'), std::env::var("HOME")) {
        (Some(rest), Ok(home)) if rest.is_empty() || rest.starts_with('/') => {
            format!("{home}{rest}")
        }
        _ => path.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_overrides_merge_per_key() {
        let mut base: Table = "[a]\nx = 1\ny = 2".parse().unwrap();
        merge(&mut base, "[a]\ny = 3".parse().unwrap());
        let s = Settings { table: base };
        assert_eq!(s.get("a.x").and_then(Value::as_integer), Some(1));
        assert_eq!(s.get("a.y").and_then(Value::as_integer), Some(3));
    }

    #[test]
    fn env_quotes_and_joins() {
        let table: Table = "[agent]\nrepo_roots = [\"/a\", \"/b\"]\nname = \"it's\""
            .parse()
            .unwrap();
        let env = Settings { table }.to_env();
        assert!(env.contains("export PCFG_AGENT_REPO_ROOTS='/a:/b'"));
        assert!(env.contains(r"export PCFG_AGENT_NAME='it'\''s'"));
    }
}
