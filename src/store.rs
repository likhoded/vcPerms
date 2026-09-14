use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::config::Config;
use crate::holder::{Group, User};
use crate::node::{purge_expired, Node};
use crate::track::Track;
use crate::util::offline_uuid;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct UuidCache {
    #[serde(flatten)]
    names: HashMap<String, String>,
}

pub struct Store {
    dir: PathBuf,
    pub config: Config,
    users: HashMap<String, User>,
    groups: HashMap<String, Group>,
    tracks: HashMap<String, Track>,
    cache: UuidCache,
    known: BTreeSet<String>,
    dirty: bool,
}

impl Store {
    pub fn load(dir: &str) -> Self {
        let dir = PathBuf::from(dir);
        let _ = std::fs::create_dir_all(dir.join("users"));
        let _ = std::fs::create_dir_all(dir.join("groups"));
        let _ = std::fs::create_dir_all(dir.join("tracks"));
        let _ = std::fs::create_dir_all(dir.join("exports"));
        let _ = std::fs::create_dir_all(dir.join("verbose"));

        let config = Config::load_or_write(dir.to_str().unwrap_or("."));
        let cache = read_json(&dir.join("uuidcache.json")).unwrap_or_default();

        let mut store = Self {
            dir,
            config,
            users: HashMap::new(),
            groups: HashMap::new(),
            tracks: HashMap::new(),
            cache,
            known: BTreeSet::new(),
            dirty: false,
        };
        store.load_all();
        if store.groups.is_empty() {
            store.ensure_group("default");
        }
        if !store.groups.contains_key(&store.config.default_group) {
            store.ensure_group(&store.config.default_group.clone());
        }
        store.load_known();
        store.ingest_holder_nodes();
        store.seed_own_nodes();
        store.save_all();
        store
    }

    fn load_all(&mut self) {
        self.users = load_map(&self.dir.join("users"), |u: &User| u.unique_id.to_ascii_lowercase());
        self.groups = load_map(&self.dir.join("groups"), |g: &Group| g.name.to_ascii_lowercase());
        self.tracks = load_map(&self.dir.join("tracks"), |t: &Track| t.name.to_ascii_lowercase());
    }

    pub fn save_all(&mut self) {
        let _ = std::fs::create_dir_all(self.dir.join("users"));
        let _ = std::fs::create_dir_all(self.dir.join("groups"));
        let _ = std::fs::create_dir_all(self.dir.join("tracks"));
        for user in self.users.values() {
            write_json(&self.user_path(&user.unique_id), user);
        }
        for group in self.groups.values() {
            write_json(&self.group_path(&group.name), group);
        }
        for track in self.tracks.values() {
            write_json(&self.track_path(&track.name), track);
        }
        write_json(&self.dir.join("uuidcache.json"), &self.cache);
        self.save_known();
        self.dirty = false;
    }

    pub fn save_if_dirty(&mut self) {
        if self.dirty {
            self.save_all();
        }
    }

    pub fn reload(&mut self) {
        self.config = Config::load_or_write(self.dir.to_str().unwrap_or("."));
        self.load_all();
        self.cache = read_json(&self.dir.join("uuidcache.json")).unwrap_or_default();
        self.load_known();
        self.ingest_holder_nodes();
        self.dirty = false;
    }

    fn load_known(&mut self) {
        if let Ok(list) = read_json::<Vec<String>>(&self.dir.join("known-permissions.json")) {
            for key in list {
                self.remember(&key);
            }
        }
    }

    fn save_known(&self) {
        let list: Vec<String> = self.known.iter().cloned().collect();
        write_json(&self.dir.join("known-permissions.json"), &list);
    }

    fn ingest_holder_nodes(&mut self) {
        let keys: Vec<String> = self
            .users
            .values()
            .flat_map(|u| u.nodes.iter().map(|n| n.key.clone()))
            .chain(self.groups.values().flat_map(|g| g.nodes.iter().map(|n| n.key.clone())))
            .collect();
        for key in keys {
            self.remember(&key);
        }
    }

    fn seed_own_nodes(&mut self) {
        for key in [
            "*",
            "vcperms.*",
            "vcperms.admin",
            "vcperms.user.info",
            "vcperms.user.permission.set",
            "vcperms.user.parent.add",
            "vcperms.user.meta.set",
            "vcperms.user.promote",
            "vcperms.user.demote",
            "vcperms.group.info",
            "vcperms.group.permission.set",
            "vcperms.group.parent.add",
            "vcperms.group.meta.set",
            "vcperms.track.info",
            "vcperms.creategroup",
            "vcperms.deletegroup",
            "vcperms.createtrack",
            "vcperms.deletetrack",
            "vcperms.verbose",
            "vcperms.import",
            "vcperms.export",
            "vcperms.reload",
            "vcperms.check",
            "vcperms.search",
            "vcPerms:command",
        ] {
            self.remember(key);
        }
    }

    pub fn remember(&mut self, key: &str) {
        let key = key.trim();
        if key.is_empty() || is_structural(key) {
            return;
        }
        if self.known.insert(key.to_string()) {
            self.dirty = true;
        }
        if let Some(star) = wildcard_parent(key) {
            if self.known.insert(star) {
                self.dirty = true;
            }
        }
    }

    pub fn remember_many<I>(&mut self, keys: I)
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        for key in keys {
            self.remember(key.as_ref());
        }
    }

    pub fn known_permissions(&self) -> Vec<String> {
        self.known.iter().cloned().collect()
    }

    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    pub fn data_dir(&self) -> &Path {
        &self.dir
    }

    pub fn user(&self, uuid: &str) -> Option<&User> {
        self.users.get(&uuid.to_ascii_lowercase())
    }

    pub fn user_mut(&mut self, uuid: &str) -> Option<&mut User> {
        self.users.get_mut(&uuid.to_ascii_lowercase())
    }

    pub fn user_by_name(&self, name: &str) -> Option<&User> {
        let n = name.to_ascii_lowercase();
        if let Some(uuid) = self.cache.names.get(&n) {
            if let Some(u) = self.users.get(&uuid.to_ascii_lowercase()) {
                return Some(u);
            }
        }
        self.users
            .values()
            .find(|u| u.name.eq_ignore_ascii_case(name))
    }

    pub fn resolve_user_id(&self, name_or_uuid: &str) -> Option<String> {
        if name_or_uuid.contains('-') && name_or_uuid.len() >= 32 {
            return Some(name_or_uuid.to_ascii_lowercase());
        }
        self.user_by_name(name_or_uuid)
            .map(|u| u.unique_id.to_ascii_lowercase())
            .or_else(|| {
                self.cache
                    .names
                    .get(&name_or_uuid.to_ascii_lowercase())
                    .cloned()
            })
    }

    pub fn ensure_user(&mut self, name: &str, uuid: Option<&str>) -> String {
        if let Some(id) = uuid {
            let id = id.to_ascii_lowercase();
            if !self.users.contains_key(&id) {
                let user = User::new(id.clone(), name.to_string(), &self.config.default_group);
                self.users.insert(id.clone(), user);
                self.dirty = true;
            } else if let Some(u) = self.users.get_mut(&id) {
                if u.name != name {
                    u.name = name.to_string();
                    self.dirty = true;
                }
            }
            self.cache.names.insert(name.to_ascii_lowercase(), id.clone());
            return id;
        }
        if let Some(existing) = self.resolve_user_id(name) {
            return existing;
        }
        let id = offline_uuid(name);
        if !self.users.contains_key(&id) {
            let user = User::new(id.clone(), name.to_string(), &self.config.default_group);
            self.users.insert(id.clone(), user);
            self.dirty = true;
        }
        self.cache.names.insert(name.to_ascii_lowercase(), id.clone());
        id
    }

    pub fn group(&self, name: &str) -> Option<&Group> {
        self.groups.get(&name.to_ascii_lowercase())
    }

    pub fn group_mut(&mut self, name: &str) -> Option<&mut Group> {
        self.groups.get_mut(&name.to_ascii_lowercase())
    }

    pub fn ensure_group(&mut self, name: &str) -> bool {
        let key = name.to_ascii_lowercase();
        if self.groups.contains_key(&key) {
            return false;
        }
        self.groups.insert(key.clone(), Group::new(&key));
        self.dirty = true;
        true
    }

    pub fn delete_group(&mut self, name: &str) -> Result<(), String> {
        let key = name.to_ascii_lowercase();
        if key == self.config.default_group {
            return Err("refusing to delete the default group".into());
        }
        if self.groups.remove(&key).is_none() {
            return Err(format!("group '{key}' does not exist"));
        }
        let group_node = format!("group.{key}");
        for user in self.users.values_mut() {
            user.nodes.retain(|n| !n.key.eq_ignore_ascii_case(&group_node));
            if user.primary_group == key {
                user.primary_group = self.config.default_group.clone();
            }
        }
        for group in self.groups.values_mut() {
            group.nodes.retain(|n| !n.key.eq_ignore_ascii_case(&group_node));
        }
        for track in self.tracks.values_mut() {
            track.remove(&key);
        }
        let _ = std::fs::remove_file(self.group_path(&key));
        self.dirty = true;
        Ok(())
    }

    pub fn rename_group(&mut self, from: &str, to: &str) -> Result<(), String> {
        let from = from.to_ascii_lowercase();
        let to = to.to_ascii_lowercase();
        if from == self.config.default_group {
            return Err("don't rename the default group, change default-group in config.yml".into());
        }
        if self.groups.contains_key(&to) {
            return Err(format!("group '{to}' already exists"));
        }
        let Some(mut group) = self.groups.remove(&from) else {
            return Err(format!("group '{from}' does not exist"));
        };
        group.name = to.clone();
        self.groups.insert(to.clone(), group);
        let old_node = format!("group.{from}");
        let new_node = format!("group.{to}");
        for user in self.users.values_mut() {
            for node in &mut user.nodes {
                if node.key.eq_ignore_ascii_case(&old_node) {
                    node.key = new_node.clone();
                }
            }
            if user.primary_group == from {
                user.primary_group = to.clone();
            }
        }
        for group in self.groups.values_mut() {
            for node in &mut group.nodes {
                if node.key.eq_ignore_ascii_case(&old_node) {
                    node.key = new_node.clone();
                }
            }
        }
        for track in self.tracks.values_mut() {
            for g in &mut track.groups {
                if g == &from {
                    *g = to.clone();
                }
            }
        }
        let _ = std::fs::remove_file(self.group_path(&from));
        self.dirty = true;
        Ok(())
    }

    pub fn clone_group(&mut self, from: &str, to: &str) -> Result<(), String> {
        let from = from.to_ascii_lowercase();
        let to = to.to_ascii_lowercase();
        let Some(src) = self.groups.get(&from).cloned() else {
            return Err(format!("group '{from}' does not exist"));
        };
        if self.groups.contains_key(&to) {
            return Err(format!("group '{to}' already exists"));
        }
        let mut copy = src;
        copy.name = to.clone();
        self.groups.insert(to, copy);
        self.dirty = true;
        Ok(())
    }

    pub fn track(&self, name: &str) -> Option<&Track> {
        self.tracks.get(&name.to_ascii_lowercase())
    }

    pub fn track_mut(&mut self, name: &str) -> Option<&mut Track> {
        self.tracks.get_mut(&name.to_ascii_lowercase())
    }

    pub fn ensure_track(&mut self, name: &str) -> bool {
        let key = name.to_ascii_lowercase();
        if self.tracks.contains_key(&key) {
            return false;
        }
        self.tracks.insert(key.clone(), Track::new(&key));
        self.dirty = true;
        true
    }

    pub fn delete_track(&mut self, name: &str) -> Result<(), String> {
        let key = name.to_ascii_lowercase();
        if self.tracks.remove(&key).is_none() {
            return Err(format!("track '{key}' does not exist"));
        }
        let _ = std::fs::remove_file(self.track_path(&key));
        self.dirty = true;
        Ok(())
    }

    pub fn rename_track(&mut self, from: &str, to: &str) -> Result<(), String> {
        let from = from.to_ascii_lowercase();
        let to = to.to_ascii_lowercase();
        if self.tracks.contains_key(&to) {
            return Err(format!("track '{to}' already exists"));
        }
        let Some(mut track) = self.tracks.remove(&from) else {
            return Err(format!("track '{from}' does not exist"));
        };
        track.name = to.clone();
        self.tracks.insert(to, track);
        let _ = std::fs::remove_file(self.track_path(&from));
        self.dirty = true;
        Ok(())
    }

    pub fn clone_track(&mut self, from: &str, to: &str) -> Result<(), String> {
        let from = from.to_ascii_lowercase();
        let to = to.to_ascii_lowercase();
        let Some(src) = self.tracks.get(&from).cloned() else {
            return Err(format!("track '{from}' does not exist"));
        };
        if self.tracks.contains_key(&to) {
            return Err(format!("track '{to}' already exists"));
        }
        let mut copy = src;
        copy.name = to.clone();
        self.tracks.insert(to, copy);
        self.dirty = true;
        Ok(())
    }

    pub fn groups(&self) -> impl Iterator<Item = &Group> {
        self.groups.values()
    }

    pub fn users(&self) -> impl Iterator<Item = &User> {
        self.users.values()
    }

    pub fn tracks(&self) -> impl Iterator<Item = &Track> {
        self.tracks.values()
    }

    pub fn group_names(&self) -> Vec<String> {
        let mut names: Vec<_> = self.groups.keys().cloned().collect();
        names.sort();
        names
    }

    pub fn track_names(&self) -> Vec<String> {
        let mut names: Vec<_> = self.tracks.keys().cloned().collect();
        names.sort();
        names
    }

    pub fn user_names(&self) -> Vec<String> {
        let mut names: Vec<_> = self.users.values().map(|u| u.name.clone()).collect();
        names.sort_by_key(|s| s.to_ascii_lowercase());
        names
    }

    pub fn permission_keys_of_group(&self, name: &str) -> Vec<String> {
        let Some(g) = self.group(name) else {
            return self.known_permissions();
        };
        merge_keys(perm_keys(&g.nodes), self.known_permissions())
    }

    pub fn permission_keys_of_user(&self, name: &str) -> Vec<String> {
        let Some(u) = self.user_by_name(name).or_else(|| self.user(name)) else {
            return self.known_permissions();
        };
        merge_keys(perm_keys(&u.nodes), self.known_permissions())
    }

    pub fn members_of(&self, group: &str) -> Vec<String> {
        let node = format!("group.{}", group.to_ascii_lowercase());
        let mut names: Vec<_> = self
            .users
            .values()
            .filter(|u| u.nodes.iter().any(|n| n.key.eq_ignore_ascii_case(&node)))
            .map(|u| u.name.clone())
            .collect();
        names.sort_by_key(|s| s.to_ascii_lowercase());
        names
    }

    pub fn clone_user(&mut self, from: &str, to: &str) -> Result<String, String> {
        let src_id = self
            .resolve_user_id(from)
            .ok_or_else(|| format!("user '{from}' is unknown"))?;
        let src = self
            .users
            .get(&src_id)
            .cloned()
            .ok_or_else(|| format!("user '{from}' is unknown"))?;
        let dest_id = self.ensure_user(to, None);
        if let Some(dest) = self.users.get_mut(&dest_id) {
            dest.primary_group = src.primary_group;
            dest.nodes = src.nodes;
        }
        self.dirty = true;
        Ok(dest_id)
    }

    pub fn sweep_expired(&mut self) -> usize {
        let mut n = 0;
        for user in self.users.values_mut() {
            n += purge_expired(&mut user.nodes);
        }
        for group in self.groups.values_mut() {
            n += purge_expired(&mut group.nodes);
        }
        if n > 0 {
            self.dirty = true;
        }
        n
    }

    pub fn anyone_has_admin(&self) -> bool {
        self.users.values().any(|u| {
            u.nodes.iter().any(|n| {
                let k = n.key.to_ascii_lowercase();
                n.value && (k == "*" || k == "vcperms.*" || k == "vcperms.admin")
            })
        }) || self.groups.values().any(|g| {
            g.nodes.iter().any(|n| {
                let k = n.key.to_ascii_lowercase();
                n.value && (k == "*" || k == "vcperms.*" || k == "vcperms.admin")
            })
        })
    }

    pub fn search(&self, query: &str) -> Vec<(String, String, String)> {
        let q = query.to_ascii_lowercase();
        let mut hits = Vec::new();
        for user in self.users.values() {
            for node in &user.nodes {
                if node.key.to_ascii_lowercase().contains(&q) {
                    hits.push((
                        format!("user/{}", user.name),
                        node.key.clone(),
                        if node.value { "true" } else { "false" }.into(),
                    ));
                }
            }
        }
        for group in self.groups.values() {
            for node in &group.nodes {
                if node.key.to_ascii_lowercase().contains(&q) {
                    hits.push((
                        format!("group/{}", group.name),
                        node.key.clone(),
                        if node.value { "true" } else { "false" }.into(),
                    ));
                }
            }
        }
        hits.sort();
        hits
    }

    pub fn export_dump(&self) -> serde_json::Value {
        serde_json::json!({
            "vcperms": env!("CARGO_PKG_VERSION"),
            "users": self.users.values().collect::<Vec<_>>(),
            "groups": self.groups.values().collect::<Vec<_>>(),
            "tracks": self.tracks.values().collect::<Vec<_>>(),
        })
    }

    pub fn import_dump(&mut self, value: serde_json::Value) -> Result<(usize, usize, usize), String> {
        let users: Vec<User> = serde_json::from_value(value.get("users").cloned().unwrap_or(serde_json::json!([])))
            .map_err(|e| format!("users: {e}"))?;
        let groups: Vec<Group> = serde_json::from_value(value.get("groups").cloned().unwrap_or(serde_json::json!([])))
            .map_err(|e| format!("groups: {e}"))?;
        let tracks: Vec<Track> = serde_json::from_value(value.get("tracks").cloned().unwrap_or(serde_json::json!([])))
            .map_err(|e| format!("tracks: {e}"))?;

        let mut u = 0;
        for user in users {
            self.cache
                .names
                .insert(user.name.to_ascii_lowercase(), user.unique_id.to_ascii_lowercase());
            self.users.insert(user.unique_id.to_ascii_lowercase(), user);
            u += 1;
        }
        let mut g = 0;
        for group in groups {
            self.groups.insert(group.name.to_ascii_lowercase(), group);
            g += 1;
        }
        let mut t = 0;
        for track in tracks {
            self.tracks.insert(track.name.to_ascii_lowercase(), track);
            t += 1;
        }
        self.dirty = true;
        Ok((u, g, t))
    }

    fn user_path(&self, uuid: &str) -> PathBuf {
        self.dir.join("users").join(format!("{}.json", uuid.to_ascii_lowercase()))
    }

    fn group_path(&self, name: &str) -> PathBuf {
        self.dir.join("groups").join(format!("{}.json", name.to_ascii_lowercase()))
    }

    fn track_path(&self, name: &str) -> PathBuf {
        self.dir.join("tracks").join(format!("{}.json", name.to_ascii_lowercase()))
    }
}

fn is_structural(key: &str) -> bool {
    let k = key.to_ascii_lowercase();
    k.starts_with("group.")
        || k.starts_with("prefix.")
        || k.starts_with("suffix.")
        || k.starts_with("meta.")
        || k.starts_with("weight.")
        || k.starts_with("displayname.")
}

fn wildcard_parent(key: &str) -> Option<String> {
    if key == "*" || key.ends_with(".*") || key.ends_with(":*") {
        return None;
    }
    if let Some((head, _)) = key.split_once('.') {
        if !head.is_empty() {
            return Some(format!("{head}.*"));
        }
    }
    None
}

fn merge_keys(mut a: Vec<String>, b: Vec<String>) -> Vec<String> {
    for key in b {
        if !a.iter().any(|x| x.eq_ignore_ascii_case(&key)) {
            a.push(key);
        }
    }
    a.sort();
    a
}

fn perm_keys(nodes: &[Node]) -> Vec<String> {
    let mut keys: Vec<String> = nodes
        .iter()
        .filter(|n| {
            !n.is_group()
                && n.prefix_parts().is_none()
                && n.suffix_parts().is_none()
                && n.meta_parts().is_none()
        })
        .map(|n| n.key.clone())
        .collect();
    keys.sort();
    keys.dedup();
    keys
}

fn load_map<T, F>(dir: &Path, key: F) -> HashMap<String, T>
where
    T: for<'de> serde::Deserialize<'de>,
    F: Fn(&T) -> String,
{
    let mut map = HashMap::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return map;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        if let Ok(item) = read_json::<T>(&path) {
            map.insert(key(&item), item);
        }
    }
    map
}

fn read_json<T: for<'de> serde::Deserialize<'de>>(path: &Path) -> Result<T, String> {
    let raw = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_json::from_str(&raw).map_err(|e| e.to_string())
}

fn write_json<T: serde::Serialize>(path: &Path, value: &T) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(raw) = serde_json::to_string_pretty(value) {
        let tmp = path.with_extension("json.tmp");
        if std::fs::write(&tmp, raw).is_ok() {
            let _ = std::fs::rename(&tmp, path);
        }
    }
}
