use std::path::Path;

#[derive(Clone, Debug)]
pub struct Config {
    pub server: String,
    pub default_group: String,
    pub allow_ops: bool,
    pub ops_override: bool,
    pub commands_allow_ops: bool,
    pub apply_chat_meta: bool,
    pub debug_logins: bool,
    pub temp_add: TempAdd,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TempAdd {
    Deny,
    Replace,
    Accumulate,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: "pumpkin".into(),
            default_group: "default".into(),
            allow_ops: true,
            ops_override: false,
            commands_allow_ops: true,
            apply_chat_meta: false,
            debug_logins: false,
            temp_add: TempAdd::Deny,
        }
    }
}

impl Config {
    pub fn load_or_write(dir: &str) -> Self {
        let path = Path::new(dir).join("config.yml");
        if let Ok(raw) = std::fs::read_to_string(&path) {
            return parse(&raw);
        }
        let _ = std::fs::write(&path, DEFAULT_YML);
        Self::default()
    }
}

const DEFAULT_YML: &str = r#"# vcPerms
# Don't flip random knobs unless you know why.

# Context value attached to every check on this instance.
server: pumpkin

# Given to every new user. Don't delete this group.
default-group: default

# If true, Pumpkin OP still works as a fallback when a node isn't set.
allow-ops: true

# If true, OP is treated as '*' unless the player has an explicit false.
# Leave this off if you actually want to use groups.
ops-override: false

# Ops can run /vcp during first-time setup. Turn off once you have
# vcperms.* on a real account.
commands-allow-ops: true

# Prefix/suffix in chat. Off by default — leave that to a chat plugin
# unless you want vcPerms to stamp names itself.
apply-chat-meta: false

debug-logins: false

# What happens if you settemp a node that already exists.
# deny | replace | accumulate
temporary-add-behaviour: deny
"#;

fn parse(raw: &str) -> Config {
    let mut cfg = Config::default();
    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((k, v)) = line.split_once(':') else {
            continue;
        };
        let key = k.trim();
        let val = v.trim().trim_matches('"');
        match key {
            "server" => cfg.server = val.to_string(),
            "default-group" => cfg.default_group = val.to_ascii_lowercase(),
            "allow-ops" => cfg.allow_ops = is_true(val),
            "ops-override" => cfg.ops_override = is_true(val),
            "commands-allow-ops" => cfg.commands_allow_ops = is_true(val),
            "apply-chat-meta" => cfg.apply_chat_meta = is_true(val),
            "debug-logins" => cfg.debug_logins = is_true(val),
            "temporary-add-behaviour" => {
                cfg.temp_add = match val.to_ascii_lowercase().as_str() {
                    "replace" => TempAdd::Replace,
                    "accumulate" => TempAdd::Accumulate,
                    _ => TempAdd::Deny,
                };
            }
            _ => {}
        }
    }
    cfg
}

fn is_true(v: &str) -> bool {
    matches!(v.to_ascii_lowercase().as_str(), "true" | "yes" | "on" | "1")
}
