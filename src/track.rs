use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Track {
    pub name: String,
    #[serde(default)]
    pub groups: Vec<String>,
}

impl Track {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_ascii_lowercase(),
            groups: Vec::new(),
        }
    }

    pub fn contains(&self, group: &str) -> bool {
        let g = group.to_ascii_lowercase();
        self.groups.iter().any(|x| x == &g)
    }

    pub fn index_of(&self, group: &str) -> Option<usize> {
        let g = group.to_ascii_lowercase();
        self.groups.iter().position(|x| x == &g)
    }

    pub fn append(&mut self, group: &str) {
        let g = group.to_ascii_lowercase();
        if !self.contains(&g) {
            self.groups.push(g);
        }
    }

    pub fn insert(&mut self, group: &str, pos: usize) {
        let g = group.to_ascii_lowercase();
        if let Some(i) = self.index_of(&g) {
            self.groups.remove(i);
        }
        let pos = pos.min(self.groups.len());
        self.groups.insert(pos, g);
    }

    pub fn remove(&mut self, group: &str) -> bool {
        let g = group.to_ascii_lowercase();
        let before = self.groups.len();
        self.groups.retain(|x| x != &g);
        self.groups.len() != before
    }

    pub fn next(&self, current: &str) -> Option<String> {
        let i = self.index_of(current)?;
        self.groups.get(i + 1).cloned()
    }

    pub fn prev(&self, current: &str) -> Option<String> {
        let i = self.index_of(current)?;
        if i == 0 {
            None
        } else {
            self.groups.get(i - 1).cloned()
        }
    }

    pub fn display(&self) -> String {
        if self.groups.is_empty() {
            "(empty)".into()
        } else {
            self.groups.join(" -> ")
        }
    }
}
