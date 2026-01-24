use std::collections::{BTreeMap, HashSet};

use super::language::TargetLanguage;
use super::naming::{escape_string, sanitize_identifier, ts_property_name, unique_name};
use super::schema::{find_object_schema, root_type_key, Schema, SchemaKind};

mod registry;
mod type_mapping;

use registry::TypeNameRegistry;

pub(super) struct TypeRenderer {
    pub(super) language: TargetLanguage,
    pub(super) registry: TypeNameRegistry,
    emitted: HashSet<String>,
    definitions: Vec<String>,
    pub(super) uses_dynamic: bool,
    pub(super) uses_structs: bool,
    pub(super) uses_list: bool,
}

impl TypeRenderer {
    pub(super) fn new(language: TargetLanguage, root_name: &str) -> Self {
        Self {
            language,
            registry: TypeNameRegistry::new(root_name),
            emitted: HashSet::new(),
            definitions: Vec::new(),
            uses_dynamic: false,
            uses_structs: false,
            uses_list: false,
        }
    }

    pub(super) fn render(mut self, schema: &Schema, root_name: &str) -> String {
        let type_key = root_type_key(schema);
        match &schema.kind {
            SchemaKind::Object(_) => {
                self.emit_object(schema, root_name);
            }
            _ => {
                if let Some((object_schema, in_array)) = find_object_schema(schema, false) {
                    let nested_name =
                        self.registry
                            .name_for_field(root_name, type_key, in_array, self.language);
                    self.emit_object(object_schema, &nested_name);
                }
                let root_alias = self.render_root_alias(schema, root_name, type_key);
                self.definitions.push(root_alias);
            }
        }

        let mut output = String::new();
        match self.language {
            TargetLanguage::Rust => {
                if self.uses_structs {
                    output.push_str("use serde::{Deserialize, Serialize};\n");
                }
                if self.uses_dynamic {
                    output.push_str("use serde_json::Value;\n");
                }
                if !output.is_empty() {
                    output.push('\n');
                }
            }
            TargetLanguage::Java => {
                if self.uses_list {
                    output.push_str("import java.util.List;\n\n");
                }
            }
            _ => {}
        }

        output.push_str(&self.definitions.join("\n\n"));
        output
    }

    fn emit_object(&mut self, schema: &Schema, name: &str) {
        let SchemaKind::Object(fields) = &schema.kind else {
            return;
        };
        if self.emitted.contains(name) {
            return;
        }

        for (key, field_schema) in fields {
            if let Some((object_schema, in_array)) = find_object_schema(field_schema, false) {
                let nested_name = self
                    .registry
                    .name_for_field(name, key, in_array, self.language);
                self.emit_object(object_schema, &nested_name);
            }
        }

        let rendered = self.render_object(name, fields);
        self.emitted.insert(name.to_string());
        self.definitions.push(rendered);
    }

    fn render_object(&mut self, name: &str, fields: &BTreeMap<String, Schema>) -> String {
        match self.language {
            TargetLanguage::Go => self.render_go_struct(name, fields),
            TargetLanguage::Rust => self.render_rust_struct(name, fields),
            TargetLanguage::TypeScript => self.render_ts_interface(name, fields),
            TargetLanguage::Dart => self.render_dart_class(name, fields),
            TargetLanguage::Java => self.render_java_class(name, fields),
            TargetLanguage::Php => self.render_php_class(name, fields),
        }
    }

    fn render_root_alias(&mut self, schema: &Schema, root_name: &str, type_key: &str) -> String {
        match self.language {
            TargetLanguage::Go => {
                let alias_type = self.go_type(schema, root_name, type_key, false);
                format!("type {} = {}", root_name, alias_type)
            }
            TargetLanguage::Rust => {
                let alias_type = self.rust_type(schema, root_name, type_key, false);
                format!("pub type {} = {};", root_name, alias_type)
            }
            TargetLanguage::TypeScript => {
                let alias_type = self.ts_type(schema, root_name, type_key, false);
                format!("type {} = {};", root_name, alias_type)
            }
            TargetLanguage::Dart => {
                let mut lines = vec![format!("class {} {{", root_name)];
                let value_type = self.dart_type(schema, root_name, type_key, false);
                let field_decl = if schema.nullable {
                    format!("  {} value;", value_type)
                } else {
                    format!("  late {} value;", value_type)
                };
                lines.push(field_decl);
                lines.push("}".to_string());
                lines.join("\n")
            }
            TargetLanguage::Java => {
                let mut lines = vec![format!("class {} {{", root_name)];
                let value_type = self.java_type(schema, root_name, type_key, false);
                lines.push(format!("  public {} value;", value_type));
                lines.push("}".to_string());
                lines.join("\n")
            }
            TargetLanguage::Php => {
                let mut lines = vec![format!("class {} {{", root_name)];
                let value_type = self.php_type(schema, root_name, type_key, false);
                lines.push(format!("  public {} $value;", value_type));
                lines.push("}".to_string());
                lines.join("\n")
            }
        }
    }

    fn render_go_struct(&mut self, name: &str, fields: &BTreeMap<String, Schema>) -> String {
        let mut lines = vec![format!("type {} struct {{", name)];
        let mut used = HashSet::new();
        for (key, schema) in fields {
            let field_name = unique_name(sanitize_identifier(key, self.language), &mut used);
            let field_type = self.go_type(schema, name, key, false);
            let tag = format!("`json:\"{}\"`", escape_string(key));
            lines.push(format!("    {} {} {}", field_name, field_type, tag));
        }
        lines.push("}".to_string());
        lines.join("\n")
    }

    fn render_rust_struct(&mut self, name: &str, fields: &BTreeMap<String, Schema>) -> String {
        self.uses_structs = true;
        let mut lines = vec![
            "#[derive(Serialize, Deserialize, Debug, Clone)]".to_string(),
            format!("pub struct {} {{", name),
        ];
        let mut used = HashSet::new();
        for (key, schema) in fields {
            let field_name = unique_name(sanitize_identifier(key, self.language), &mut used);
            if field_name != *key {
                lines.push(format!("    #[serde(rename = \"{}\")]", escape_string(key)));
            }
            let field_type = self.rust_type(schema, name, key, false);
            lines.push(format!("    pub {}: {},", field_name, field_type));
        }
        lines.push("}".to_string());
        lines.join("\n")
    }

    fn render_ts_interface(&mut self, name: &str, fields: &BTreeMap<String, Schema>) -> String {
        let mut lines = vec![format!("interface {} {{", name)];
        for (key, schema) in fields {
            let (prop_name, quoted) = ts_property_name(key);
            let rendered = if quoted {
                format!("\"{}\"", escape_string(&prop_name))
            } else {
                prop_name
            };
            let field_type = self.ts_type(schema, name, key, false);
            lines.push(format!("  {}: {};", rendered, field_type));
        }
        lines.push("}".to_string());
        lines.join("\n")
    }

    fn render_dart_class(&mut self, name: &str, fields: &BTreeMap<String, Schema>) -> String {
        let mut lines = vec![format!("class {} {{", name)];
        let mut used = HashSet::new();
        for (key, schema) in fields {
            let field_name = unique_name(sanitize_identifier(key, self.language), &mut used);
            if field_name != *key {
                lines.push(format!("  // json: \"{}\"", escape_string(key)));
            }
            let field_type = self.dart_type(schema, name, key, false);
            let decl = if schema.nullable {
                format!("  {} {};", field_type, field_name)
            } else {
                format!("  late {} {};", field_type, field_name)
            };
            lines.push(decl);
        }
        lines.push("}".to_string());
        lines.join("\n")
    }

    fn render_java_class(&mut self, name: &str, fields: &BTreeMap<String, Schema>) -> String {
        let mut lines = vec![format!("class {} {{", name)];
        let mut used = HashSet::new();
        for (key, schema) in fields {
            let field_name = unique_name(sanitize_identifier(key, self.language), &mut used);
            if field_name != *key {
                lines.push(format!("  // json: \"{}\"", escape_string(key)));
            }
            let field_type = self.java_type(schema, name, key, false);
            lines.push(format!("  public {} {};", field_type, field_name));
        }
        lines.push("}".to_string());
        lines.join("\n")
    }

    fn render_php_class(&mut self, name: &str, fields: &BTreeMap<String, Schema>) -> String {
        let mut lines = vec![format!("class {} {{", name)];
        let mut used = HashSet::new();
        for (key, schema) in fields {
            let field_name = unique_name(sanitize_identifier(key, self.language), &mut used);
            if field_name != *key {
                lines.push(format!("  // json: \"{}\"", escape_string(key)));
            }
            let field_type = self.php_type(schema, name, key, false);
            lines.push(format!("  public {} ${};", field_type, field_name));
        }
        lines.push("}".to_string());
        lines.join("\n")
    }
}
