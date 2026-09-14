use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tracing::error;

use crate::config::Config;
use crate::holder::{Group, Holder, User};
use crate::node::{purge_expired, Node};
use crate::resolve::primary_from_parents;
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
    user_ids: HashSet<String>,
    groups: HashMap<String, Group>,
    tracks: HashMap<String, Track>,
    cache: UuidCache,
    known: BTreeSet<String>,
    last_world: HashMap<String, String>,
    last_dimension: HashMap<String, String>,
    dirty_users: HashSet<String>,
    dirty_groups: HashSet<String>,
    dirty_tracks: HashSet<String>,
    dirty_known: bool,
    dirty_cache: bool,
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
            user_ids: HashSet::new(),
            groups: HashMap::new(),
            tracks: HashMap::new(),
            cache,
            known: BTreeSet::new(),
            last_world: HashMap::new(),
            last_dimension: HashMap::new(),
            dirty_users: HashSet::new(),
            dirty_groups: HashSet::new(),
            dirty_tracks: HashSet::new(),
            dirty_known: false,
            dirty_cache: false,
        };
        store.index_user_files();
        store.groups = load_map(&store.dir.join("groups"), |g: &Group| g.name.to_ascii_lowercase());
        store.tracks = load_map(&store.dir.join("tracks"), |t: &Track| t.name.to_ascii_lowercase());
        if store.groups.is_empty() {
            store.ensure_group("default");
        }
        if !store.groups.contains_key(&store.config.default_group) {
            store.ensure_group(&store.config.default_group.clone());
        }
        store.load_known();
        store.ingest_holder_nodes();
        store.seed_own_nodes();
        store.save_if_dirty();
        store
    }

    fn index_user_files(&mut self) {
        self.user_ids = list_json_stems(&self.dir.join("users"));
    }

    pub fn load_user(&mut self, uuid: &str) {
        let id = uuid.to_ascii_lowercase();
        if self.users.contains_key(&id) {
            return;
        }
        let path = self.user_path(&id);
        match read_json::<User>(&path) {
            Ok(mut user) => {
                let n = purge_expired(&mut user.nodes);
                let keys: Vec<String> = user.nodes.iter().map(|node| node.key.clone()).collect();
                self.users.insert(id.clone(), user);
                self.user_ids.insert(id.clone());
                for key in keys {
                    self.remember(&key);
                }
                if n > 0 {
                    self.touch_user(&id);
                }
            }
            Err(e) if path.exists() => {
                error!("vcPerms skipped broken user {}: {e}", path.display());
            }
            Err(_) => {}
        }
    }

    pub fn load_all_users(&mut self) {
        let ids: Vec<String> = self.user_ids.iter().cloned().collect();
        for id in ids {
            self.load_user(&id);
        }
    }

    pub fn remember_location(&mut self, uuid: &str, world: &str, dimension: &str) {
        let id = uuid.to_ascii_lowercase();
        self.last_world.insert(id.clone(), world.to_string());
        self.last_dimension.insert(id, dimension.to_string());
    }

    pub fn forget_location(&mut self, uuid: &str) {
        let id = uuid.to_ascii_lowercase();
        self.last_world.remove(&id);
        self.last_dimension.remove(&id);
    }

    pub fn last_world(&self, uuid: &str) -> Option<&str> {
        self.last_world.get(&uuid.to_ascii_lowercase()).map(String::as_str)
    }

    pub fn last_dimension(&self, uuid: &str) -> Option<&str> {
        self.last_dimension.get(&uuid.to_ascii_lowercase()).map(String::as_str)
    }

    pub fn unload_user(&mut self, uuid: &str) {
        let id = uuid.to_ascii_lowercase();
        if self.dirty_users.contains(&id) {
            self.save_if_dirty();
        }
        self.users.remove(&id);
        self.forget_location(&id);
    }

    pub fn save_all(&mut self) {
        self.load_all_users();
        self.dirty_users.extend(self.users.keys().cloned());
        self.dirty_groups.extend(self.groups.keys().cloned());
        self.dirty_tracks.extend(self.tracks.keys().cloned());
        self.dirty_known = true;
        self.dirty_cache = true;
        self.save_if_dirty();
    }

    pub fn save_if_dirty(&mut self) {
        let _ = std::fs::create_dir_all(self.dir.join("users"));
        let _ = std::fs::create_dir_all(self.dir.join("groups"));
        let _ = std::fs::create_dir_all(self.dir.join("tracks"));
        let users: Vec<String> = self.dirty_users.drain().collect();
        for id in users {
            if let Some(user) = self.users.get(&id) {
                write_json(&self.user_path(&user.unique_id), user);
            }
        }
        let groups: Vec<String> = self.dirty_groups.drain().collect();
        for name in groups {
            if let Some(group) = self.groups.get(&name) {
                write_json(&self.group_path(&group.name), group);
            }
        }
        let tracks: Vec<String> = self.dirty_tracks.drain().collect();
        for name in tracks {
            if let Some(track) = self.tracks.get(&name) {
                write_json(&self.track_path(&track.name), track);
            }
        }
        if self.dirty_cache {
            write_json(&self.dir.join("uuidcache.json"), &self.cache);
            self.dirty_cache = false;
        }
        if self.dirty_known {
            self.save_known();
            self.dirty_known = false;
        }
    }

    pub fn reload(&mut self) {
        self.config = Config::load_or_write(self.dir.to_str().unwrap_or("."));
        self.users.clear();
        self.index_user_files();
        self.groups = load_map(&self.dir.join("groups"), |g: &Group| g.name.to_ascii_lowercase());
        self.tracks = load_map(&self.dir.join("tracks"), |t: &Track| t.name.to_ascii_lowercase());
        self.cache = read_json(&self.dir.join("uuidcache.json")).unwrap_or_default();
        self.load_known();
        self.ingest_holder_nodes();
        self.dirty_users.clear();
        self.dirty_groups.clear();
        self.dirty_tracks.clear();
        self.dirty_known = false;
        self.dirty_cache = false;
    }

    fn load_known(&mut self) {
        if let Ok(list) = read_json::<Vec<String>>(&self.dir.join("known-permissions.json")) {
            for key in list {
                let _ = self.known.insert(key);
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
            "vcperms.user.permission.unset",
            "vcperms.user.permission.check",
            "vcperms.user.parent.add",
            "vcperms.user.parent.remove",
            "vcperms.user.meta.set",
            "vcperms.user.meta.unset",
            "vcperms.user.promote",
            "vcperms.user.demote",
            "vcperms.group.info",
            "vcperms.group.permission.set",
            "vcperms.group.permission.unset",
            "vcperms.group.parent.add",
            "vcperms.group.parent.remove",
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
            self.dirty_known = true;
        }
        if let Some(star) = wildcard_parent(key) {
            if self.known.insert(star) {
                self.dirty_known = true;
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

    pub fn knows(&self, key: &str) -> bool {
        self.known.contains(key)
    }

    pub fn indexed_user_count(&self) -> usize {
        self.user_ids.len().max(self.users.len())
    }

    pub fn load_named(&mut self, name_or_uuid: &str) -> Option<String> {
        if name_or_uuid.contains('-') && name_or_uuid.len() >= 32 {
            let id = name_or_uuid.to_ascii_lowercase();
            self.load_user(&id);
            if self.users.contains_key(&id) {
                return Some(id);
            }
        }
        let id = self.resolve_user_id(name_or_uuid)?;
        self.load_user(&id);
        self.users.contains_key(&id).then_some(id)
    }

    pub fn refresh_user_primary(&mut self, uuid: &str) {
        self.refresh_primary(uuid);
    }

    pub fn touch_user(&mut self, uuid: &str) {
        self.dirty_users.insert(uuid.to_ascii_lowercase());
    }

    pub fn touch_group(&mut self, name: &str) {
        self.dirty_groups.insert(name.to_ascii_lowercase());
    }

    pub fn touch_track(&mut self, name: &str) {
        self.dirty_tracks.insert(name.to_ascii_lowercase());
    }

    fn refresh_primary(&mut self, uuid: &str) {
        let parents = self
            .users
            .get(&uuid.to_ascii_lowercase())
            .map(|u| u.parents())
            .unwrap_or_default();
        let primary = primary_from_parents(self, &parents);
        if let Some(user) = self.users.get_mut(&uuid.to_ascii_lowercase()) {
            if user.primary_group != primary {
                user.primary_group = primary;
                self.touch_user(uuid);
            }
        }
    }

    pub fn data_dir(&self) -> &Path {
        &self.dir
    }

    pub fn user(&self, uuid: &str) -> Option<&User> {
        self.users.get(&uuid.to_ascii_lowercase())
    }

    pub fn user_mut(&mut self, uuid: &str) -> Option<&mut User> {
        self.load_user(uuid);
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
            self.load_user(&id);
            self.merge_name_ghosts(name, &id);
            if !self.users.contains_key(&id) {
                let user = User::new(id.clone(), name.to_string(), &self.config.default_group);
                self.users.insert(id.clone(), user);
                self.user_ids.insert(id.clone());
                self.touch_user(&id);
            } else if let Some(u) = self.users.get_mut(&id) {
                if u.name != name {
                    u.name = name.to_string();
                    self.touch_user(&id);
                }
            }
            self.cache.names.insert(name.to_ascii_lowercase(), id.clone());
            self.dirty_cache = true;
            return id;
        }
        if let Some(existing) = self.resolve_user_id(name) {
            self.load_user(&existing);
            return existing;
        }
        let id = offline_uuid(name);
        self.load_user(&id);
        if !self.users.contains_key(&id) {
            let user = User::new(id.clone(), name.to_string(), &self.config.default_group);
            self.users.insert(id.clone(), user);
            self.user_ids.insert(id.clone());
            self.touch_user(&id);
        }
        self.cache.names.insert(name.to_ascii_lowercase(), id.clone());
        self.dirty_cache = true;
        id
    }

    fn merge_name_ghosts(&mut self, name: &str, real_id: &str) {
        let mut ghosts = Vec::new();
        let offline = offline_uuid(name);
        if offline != real_id {
            ghosts.push(offline);
        }
        if let Some(cached) = self.cache.names.get(&name.to_ascii_lowercase()).cloned() {
            if cached != real_id && !ghosts.contains(&cached) {
                ghosts.push(cached);
            }
        }
        for ghost_id in ghosts {
            self.load_user(&ghost_id);
            let Some(ghost) = self.users.remove(&ghost_id) else {
                continue;
            };
            if !ghost.name.eq_ignore_ascii_case(name) {
                self.users.insert(ghost_id, ghost);
                continue;
            }
            self.load_user(real_id);
            if !self.users.contains_key(real_id) {
                let mut moved = ghost;
                moved.unique_id = real_id.to_string();
                moved.name = name.to_string();
                self.users.insert(real_id.to_string(), moved);
                self.user_ids.insert(real_id.to_string());
            } else if let Some(dest) = self.users.get_mut(real_id) {
                for node in ghost.nodes {
                    dest.add_node(node);
                }
                dest.name = name.to_string();
            }
            let _ = std::fs::remove_file(self.user_path(&ghost_id));
            self.user_ids.remove(&ghost_id);
            self.touch_user(real_id);
        }
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
        self.touch_group(&key);
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
        self.load_all_users();
        let group_node = format!("group.{key}");
        let user_ids: Vec<String> = self.users.keys().cloned().collect();
        for user in self.users.values_mut() {
            user.nodes.retain(|n| !n.key.eq_ignore_ascii_case(&group_node));
            if user.primary_group == key {
                user.primary_group = self.config.default_group.clone();
            }
        }
        for id in &user_ids {
            self.touch_user(id);
        }
        let group_ids: Vec<String> = self.groups.keys().cloned().collect();
        for group in self.groups.values_mut() {
            group.nodes.retain(|n| !n.key.eq_ignore_ascii_case(&group_node));
        }
        for g in &group_ids {
            self.touch_group(g);
        }
        let track_ids: Vec<String> = self.tracks.keys().cloned().collect();
        for track in self.tracks.values_mut() {
            track.remove(&key);
        }
        for t in &track_ids {
            self.touch_track(t);
        }
        let _ = std::fs::remove_file(self.group_path(&key));
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
        self.load_all_users();
        let old_node = format!("group.{from}");
        let new_node = format!("group.{to}");
        let user_ids: Vec<String> = self.users.keys().cloned().collect();
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
        for id in &user_ids {
            self.touch_user(id);
        }
        let group_ids: Vec<String> = self.groups.keys().cloned().collect();
        for group in self.groups.values_mut() {
            for node in &mut group.nodes {
                if node.key.eq_ignore_ascii_case(&old_node) {
                    node.key = new_node.clone();
                }
            }
        }
        for g in &group_ids {
            self.touch_group(g);
        }
        let track_ids: Vec<String> = self.tracks.keys().cloned().collect();
        for track in self.tracks.values_mut() {
            for g in &mut track.groups {
                if g == &from {
                    *g = to.clone();
                }
            }
        }
        for t in &track_ids {
            self.touch_track(t);
        }
        let _ = std::fs::remove_file(self.group_path(&from));
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
        self.groups.insert(to.clone(), copy);
        self.touch_group(&to);
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
        self.touch_track(&key);
        true
    }

    pub fn delete_track(&mut self, name: &str) -> Result<(), String> {
        let key = name.to_ascii_lowercase();
        if self.tracks.remove(&key).is_none() {
            return Err(format!("track '{key}' does not exist"));
        }
        let _ = std::fs::remove_file(self.track_path(&key));
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
        self.tracks.insert(to.clone(), track);
        let _ = std::fs::remove_file(self.track_path(&from));
        self.touch_track(&to);
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
        self.tracks.insert(to.clone(), copy);
        self.touch_track(&to);
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
        let mut names: Vec<String> = self.cache.names.keys().cloned().collect();
        for user in self.users.values() {
            if !names.iter().any(|n| n.eq_ignore_ascii_case(&user.name)) {
                names.push(user.name.clone());
            }
        }
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

    pub fn members_of(&mut self, group: &str) -> Vec<String> {
        self.load_all_users();
        let node = format!("group.{}", group.to_ascii_lowercase());
        let mut names: Vec<_> = self
            .users
            .values()
            .filter(|u| {
                u.nodes
                    .iter()
                    .any(|n| n.value && n.key.eq_ignore_ascii_case(&node))
            })
            .map(|u| u.name.clone())
            .collect();
        names.sort_by_key(|s| s.to_ascii_lowercase());
        names
    }

    pub fn clone_user(&mut self, from: &str, to: &str) -> Result<String, String> {
        let src_id = self
            .resolve_user_id(from)
            .ok_or_else(|| format!("user '{from}' is unknown"))?;
        self.load_user(&src_id);
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
        self.touch_user(&dest_id);
        Ok(dest_id)
    }

    pub fn sweep_expired(&mut self) -> usize {
        let mut n = 0;
        let mut dirty_users = Vec::new();
        for (id, user) in &mut self.users {
            let dropped = purge_expired(&mut user.nodes);
            if dropped > 0 {
                dirty_users.push(id.clone());
                n += dropped;
            }
        }
        for id in dirty_users {
            self.touch_user(&id);
        }
        let mut dirty_groups = Vec::new();
        for (name, group) in &mut self.groups {
            let dropped = purge_expired(&mut group.nodes);
            if dropped > 0 {
                dirty_groups.push(name.clone());
                n += dropped;
            }
        }
        for name in dirty_groups {
            self.touch_group(&name);
        }
        n
    }

    pub fn anyone_has_admin(&mut self) -> bool {
        let group_hit = self.groups.values().any(|g| {
            g.nodes.iter().any(|n| {
                let k = n.key.to_ascii_lowercase();
                n.value && (k == "*" || k == "vcperms.*" || k == "vcperms.admin")
            })
        });
        if group_hit {
            return true;
        }
        self.load_all_users();
        self.users.values().any(|u| {
            u.nodes.iter().any(|n| {
                let k = n.key.to_ascii_lowercase();
                n.value && (k == "*" || k == "vcperms.*" || k == "vcperms.admin")
            })
        })
    }

    pub fn search(&mut self, query: &str) -> Vec<(String, String, String)> {
        self.load_all_users();
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

    pub fn export_dump(&mut self) -> serde_json::Value {
        self.load_all_users();
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
            let id = user.unique_id.to_ascii_lowercase();
            self.cache
                .names
                .insert(user.name.to_ascii_lowercase(), id.clone());
            self.users.insert(id.clone(), user);
            self.user_ids.insert(id.clone());
            self.touch_user(&id);
            u += 1;
        }
        let mut g = 0;
        for group in groups {
            let name = group.name.to_ascii_lowercase();
            self.groups.insert(name.clone(), group);
            self.touch_group(&name);
            g += 1;
        }
        let mut t = 0;
        for track in tracks {
            let name = track.name.to_ascii_lowercase();
            self.tracks.insert(name.clone(), track);
            self.touch_track(&name);
            t += 1;
        }
        self.dirty_cache = true;
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

#[cfg(test)]
impl Store {
    pub(crate) fn test_empty() -> Self {
        Self {
            dir: PathBuf::from("."),
            config: Config::default(),
            users: HashMap::new(),
            user_ids: HashSet::new(),
            groups: HashMap::new(),
            tracks: HashMap::new(),
            cache: UuidCache::default(),
            known: BTreeSet::new(),
            last_world: HashMap::new(),
            last_dimension: HashMap::new(),
            dirty_users: HashSet::new(),
            dirty_groups: HashSet::new(),
            dirty_tracks: HashSet::new(),
            dirty_known: false,
            dirty_cache: false,
        }
    }

    pub(crate) fn test_insert_user(&mut self, user: User) {
        self.users.insert(user.unique_id.to_ascii_lowercase(), user);
    }

    pub(crate) fn test_insert_group(&mut self, group: Group) {
        self.groups.insert(group.name.to_ascii_lowercase(), group);
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
    if let Some((head, _)) = key.split_once(':') {
        if !head.is_empty() {
            return Some(format!("{head}:*"));
        }
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
        match read_json::<T>(&path) {
            Ok(item) => {
                map.insert(key(&item), item);
            }
            Err(e) => {
                error!("vcPerms skipped broken json {}: {e}", path.display());
            }
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
    let Ok(raw) = serde_json::to_string_pretty(value) else {
        error!("vcPerms failed to serialize {}", path.display());
        return;
    };
    let tmp = path.with_extension("json.tmp");
    if let Err(e) = std::fs::write(&tmp, &raw) {
        error!("vcPerms failed to write {}: {e}", tmp.display());
        return;
    }
    if path.exists() {
        if let Err(e) = std::fs::remove_file(path) {
            error!("vcPerms could not replace {}: {e}", path.display());
            if let Err(e2) = std::fs::write(path, &raw) {
                error!("vcPerms fallback write failed {}: {e2}", path.display());
            }
            let _ = std::fs::remove_file(&tmp);
            return;
        }
    }
    if let Err(e) = std::fs::rename(&tmp, path) {
        error!(
            "vcPerms rename failed {} -> {}: {e}",
            tmp.display(),
            path.display()
        );
        if let Err(e2) = std::fs::write(path, &raw) {
            error!("vcPerms fallback write failed {}: {e2}", path.display());
        }
        let _ = std::fs::remove_file(&tmp);
    }
}

fn list_json_stems(dir: &Path) -> HashSet<String> {
    let mut set = HashSet::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return set;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
            set.insert(stem.to_ascii_lowercase());
        }
    }
    set
}
