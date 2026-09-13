use std::collections::VecDeque;

use serde::Serialize;

use crate::util::now_secs;

const CAP: usize = 2000;

#[derive(Clone, Debug, Serialize)]
pub struct VerboseHit {
    pub time: u64,
    pub player: String,
    pub permission: String,
    pub result: bool,
    pub source: String,
    pub node: String,
}

#[derive(Clone, Debug)]
pub struct Verbose {
    pub enabled: bool,
    pub filter: Option<String>,
    hits: VecDeque<VerboseHit>,
}

impl Default for Verbose {
    fn default() -> Self {
        Self {
            enabled: false,
            filter: None,
            hits: VecDeque::new(),
        }
    }
}

impl Verbose {
    pub fn record(&mut self, hit: VerboseHit) {
        if !self.enabled {
            return;
        }
        if let Some(f) = &self.filter {
            let f = f.to_ascii_lowercase();
            if !hit.player.to_ascii_lowercase().contains(&f)
                && !hit.permission.to_ascii_lowercase().contains(&f)
            {
                return;
            }
        }
        if self.hits.len() >= CAP {
            self.hits.pop_front();
        }
        self.hits.push_back(hit);
    }

    pub fn clear(&mut self) {
        self.hits.clear();
    }

    pub fn snapshot(&self) -> Vec<VerboseHit> {
        self.hits.iter().cloned().collect()
    }

    pub fn hit(player: String, permission: String, result: bool, source: String, node: String) -> VerboseHit {
        VerboseHit {
            time: now_secs(),
            player,
            permission,
            result,
            source,
            node,
        }
    }
}
