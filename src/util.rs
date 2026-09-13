use md5::{Digest, Md5};
use pumpkin_plugin_api::player::Player;
use pumpkin_plugin_api::text::TextComponent;
use pumpkin_plugin_api::uuid::Uuid;

pub fn legacy(s: &str) -> TextComponent {
    TextComponent::from_legacy_string_with_code(s, '&')
}

pub fn player_uuid(player: &Player) -> String {
    format_uuid(player.get_id())
}

pub fn format_uuid(id: Uuid) -> String {
    id.to_string()
}

/// Same algorithm Java uses for offline-mode players.
pub fn offline_uuid(name: &str) -> String {
    let mut hasher = Md5::new();
    hasher.update(format!("OfflinePlayer:{name}").as_bytes());
    let mut bytes = hasher.finalize();
    bytes[6] = (bytes[6] & 0x0f) | 0x30;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes[0],
        bytes[1],
        bytes[2],
        bytes[3],
        bytes[4],
        bytes[5],
        bytes[6],
        bytes[7],
        bytes[8],
        bytes[9],
        bytes[10],
        bytes[11],
        bytes[12],
        bytes[13],
        bytes[14],
        bytes[15]
    )
}

pub fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub fn tokenize(input: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quote = false;
    for ch in input.chars() {
        match ch {
            '"' => quote = !quote,
            c if c.is_whitespace() && !quote => {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
            }
            _ => cur.push(ch),
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

pub fn parse_bool(raw: &str) -> Option<bool> {
    match raw.to_ascii_lowercase().as_str() {
        "true" | "yes" | "on" | "1" => Some(true),
        "false" | "no" | "off" | "0" => Some(false),
        _ => None,
    }
}

pub fn parse_duration(raw: &str) -> Result<u64, String> {
    let s = raw.trim().to_ascii_lowercase();
    if s.is_empty() {
        return Err("empty duration".into());
    }
    if s == "0" || s == "0s" {
        return Err("duration must be > 0".into());
    }

    let mut total = 0u64;
    let mut num = String::new();
    let mut unit = String::new();
    let flush = |num: &mut String, unit: &mut String, total: &mut u64| -> Result<(), String> {
        if num.is_empty() {
            return if unit.is_empty() {
                Ok(())
            } else {
                Err(format!("bad duration: {raw}"))
            };
        }
        let n: u64 = num
            .parse()
            .map_err(|_| format!("bad number in duration: {raw}"))?;
        let mul = match unit.as_str() {
            "" | "s" | "sec" | "secs" | "second" | "seconds" => 1,
            "m" | "min" | "mins" | "minute" | "minutes" => 60,
            "h" | "hr" | "hrs" | "hour" | "hours" => 3600,
            "d" | "day" | "days" => 86400,
            "w" | "week" | "weeks" => 604800,
            "mo" | "month" | "months" => 2592000,
            "y" | "yr" | "year" | "years" => 31536000,
            other => return Err(format!("unknown duration unit '{other}'")),
        };
        *total = total.saturating_add(n.saturating_mul(mul));
        num.clear();
        unit.clear();
        Ok(())
    };

    for ch in s.chars() {
        if ch.is_ascii_digit() {
            if !unit.is_empty() {
                flush(&mut num, &mut unit, &mut total)?;
            }
            num.push(ch);
        } else if ch.is_ascii_alphabetic() {
            unit.push(ch);
        } else {
            return Err(format!("bad duration: {raw}"));
        }
    }
    flush(&mut num, &mut unit, &mut total)?;
    if total == 0 {
        return Err("duration must be > 0".into());
    }
    Ok(total)
}

pub fn fmt_duration(secs: u64) -> String {
    if secs == 0 {
        return "0s".into();
    }
    let mut left = secs;
    let mut parts = Vec::new();
    for (unit, size) in [("y", 31536000u64), ("mo", 2592000), ("w", 604800), ("d", 86400), ("h", 3600), ("m", 60)] {
        if left >= size {
            parts.push(format!("{}{unit}", left / size));
            left %= size;
        }
    }
    if left > 0 || parts.is_empty() {
        parts.push(format!("{left}s"));
    }
    parts.join("")
}

pub fn fmt_remaining(expiry: u64, now: u64) -> String {
    if expiry <= now {
        "expired".into()
    } else {
        fmt_duration(expiry - now)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duration_combo() {
        assert_eq!(parse_duration("1d2h").unwrap(), 86400 + 7200);
        assert_eq!(parse_duration("30m").unwrap(), 1800);
    }

    #[test]
    fn quotes() {
        assert_eq!(
            tokenize(r#"meta addprefix 100 "&c[Admin] ""#),
            ["meta", "addprefix", "100", "&c[Admin] "]
        );
    }
}
