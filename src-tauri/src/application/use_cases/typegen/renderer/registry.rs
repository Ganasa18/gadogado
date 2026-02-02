use std::collections::{HashMap, HashSet};

use super::super::language::TargetLanguage;
use super::super::naming::{sanitize_type_name, to_pascal_case, unique_name};

pub(super) struct TypeNameRegistry {
    used: HashSet<String>,
    assigned: HashMap<String, String>,
}

impl TypeNameRegistry {
    pub(super) fn new(root: &str) -> Self {
        let mut used = HashSet::new();
        used.insert(root.to_string());
        Self {
            used,
            assigned: HashMap::new(),
        }
    }

    pub(super) fn name_for_field(
        &mut self,
        parent: &str,
        key: &str,
        in_array: bool,
        language: TargetLanguage,
    ) -> String {
        let id = format!(
            "{}::{}::{}",
            parent,
            key,
            if in_array { "item" } else { "obj" }
        );
        if let Some(name) = self.assigned.get(&id) {
            return name.clone();
        }

        let mut base = format!("{}{}", parent, to_pascal_case(key));
        if in_array && !base.ends_with("Item") {
            base.push_str("Item");
        }

        let base = sanitize_type_name(&base, language);
        let name = unique_name(base, &mut self.used);
        self.assigned.insert(id, name.clone());
        name
    }
}
