use crate::domain::error::{AppError, Result};
use crate::domain::llm_config::LLMConfig;
use crate::domain::typegen::TypeGenMode;
use crate::infrastructure::llm_clients::LLMClient;
use crate::infrastructure::response::clean_llm_response;
use serde_json::Value;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;

const MAX_JSON_CHARS: usize = 16_384;

pub struct TypeGenUseCase {
    llm_client: Arc<dyn LLMClient + Send + Sync>,
}

impl TypeGenUseCase {
    pub fn new(llm_client: Arc<dyn LLMClient + Send + Sync>) -> Self {
        Self { llm_client }
    }

    pub async fn execute(
        &self,
        config: &LLMConfig,
        json_input: String,
        language: String,
        root_name: String,
        mode: TypeGenMode,
    ) -> Result<String> {
        let sanitized = strip_control_chars(&json_input);
        if sanitized.len() > MAX_JSON_CHARS {
            return Err(AppError::ValidationError(format!(
                "JSON exceeds {} characters.",
                MAX_JSON_CHARS
            )));
        }

        let parsed: Value = serde_json::from_str(&sanitized).map_err(|e| {
            AppError::ValidationError(format!("Invalid JSON input: {}", e))
        })?;
        let schema = infer_schema(&parsed);

        let pretty_json = serde_json::to_string_pretty(&parsed).unwrap_or_else(|_| sanitized);
        let root = if root_name.trim().is_empty() {
            "Root".to_string()
        } else {
            root_name.trim().to_string()
        };
        let language = language.trim().to_string();

        match mode {
            TypeGenMode::Offline => generate_offline(&schema, &root, &language),
            TypeGenMode::Llm => {
                generate_llm(self, config, &pretty_json, &language, &root).await
            }
            TypeGenMode::Auto => {
                let llm_result = generate_llm(self, config, &pretty_json, &language, &root).await;
                match llm_result {
                    Ok(result) => Ok(result),
                    Err(llm_err) => {
                        match generate_offline(&schema, &root, &language) {
                            Ok(result) => Ok(result),
                            Err(_) => Err(llm_err),
                        }
                    }
                }
            }
        }
    }
}

async fn generate_llm(
    use_case: &TypeGenUseCase,
    config: &LLMConfig,
    pretty_json: &str,
    language: &str,
    root_name: &str,
) -> Result<String> {
    let system_prompt = build_system_prompt(language, root_name);
    let user_prompt = format!("JSON:\n{}", pretty_json);

    let raw_result = use_case
        .llm_client
        .generate(config, &system_prompt, &user_prompt)
        .await?;

    Ok(clean_llm_response(&raw_result))
}

fn generate_offline(schema: &Schema, root_name: &str, language: &str) -> Result<String> {
    let lang = TargetLanguage::parse(language).ok_or_else(|| {
        AppError::ValidationError(format!(
            "Unsupported language for offline mode: {}",
            language
        ))
    })?;
    let root_name = sanitize_type_name(root_name, lang);
    let renderer = TypeRenderer::new(lang, &root_name);
    Ok(renderer.render(schema, &root_name))
}

fn strip_control_chars(input: &str) -> String {
    input.chars().filter(|ch| (*ch as u32) >= 0x20).collect()
}

fn build_system_prompt(language: &str, root_name: &str) -> String {
    format!(
        "You are a code generator. Generate type definitions from a JSON response.\n\
Target language: {language}\n\
Root type name: {root_name}\n\
Requirements:\n\
- Output ONLY code. No markdown, no explanations.\n\
- Define all nested object and array types.\n\
- Use idiomatic naming for types in the target language.\n\
- Preserve JSON field names. If a field name is not a valid identifier, use a safe identifier and add a mapping using the target language's conventions.\n\
- Use nullable/optional types when JSON values can be null.\n\
- Treat all JSON strings as plain string types; do not infer date/time types.\n\
- Do not include parsing, constructors, or helper functions.\n\
Language rules:\n\
- Go: use struct types and json tags.\n\
- Rust: use struct types with serde derive and serde rename when needed.\n\
- TypeScript: use interfaces and quoted property names when needed.\n\
- Dart: use class types with typed fields only; if needed, add a comment like: // json: \"original\".\n\
- Flutter: use Dart class types with typed fields only; if needed, add a comment like: // json: \"original\".\n\
- Java: use class types with public fields; use List<T> for arrays and boxed types when needed.\n\
- PHP: use class types with typed public properties; if needed, add a comment like: // json: \"original\".\n\
"
    )
}

#[derive(Clone, Debug)]
struct Schema {
    kind: SchemaKind,
    nullable: bool,
}

#[derive(Clone, Debug)]
enum SchemaKind {
    Bool,
    Int,
    Float,
    String,
    Array(Box<Schema>),
    Object(BTreeMap<String, Schema>),
    Dynamic,
}

fn infer_schema(value: &Value) -> Schema {
    match value {
        Value::Null => Schema {
            kind: SchemaKind::Dynamic,
            nullable: true,
        },
        Value::Bool(_) => Schema {
            kind: SchemaKind::Bool,
            nullable: false,
        },
        Value::Number(num) => Schema {
            kind: if num.is_i64() || num.is_u64() {
                SchemaKind::Int
            } else {
                SchemaKind::Float
            },
            nullable: false,
        },
        Value::String(_) => Schema {
            kind: SchemaKind::String,
            nullable: false,
        },
        Value::Array(items) => {
            let mut merged: Option<Schema> = None;
            for item in items {
                let item_schema = infer_schema(item);
                merged = Some(match merged {
                    Some(existing) => merge_schema(existing, item_schema),
                    None => item_schema,
                });
            }
            let element_schema = merged.unwrap_or(Schema {
                kind: SchemaKind::Dynamic,
                nullable: false,
            });
            Schema {
                kind: SchemaKind::Array(Box::new(element_schema)),
                nullable: false,
            }
        }
        Value::Object(map) => {
            let mut fields = BTreeMap::new();
            for (key, value) in map {
                fields.insert(key.clone(), infer_schema(value));
            }
            Schema {
                kind: SchemaKind::Object(fields),
                nullable: false,
            }
        }
    }
}

fn merge_schema(a: Schema, b: Schema) -> Schema {
    let nullable = a.nullable || b.nullable;
    match (a.kind, b.kind) {
        (SchemaKind::Dynamic, _) | (_, SchemaKind::Dynamic) => Schema {
            kind: SchemaKind::Dynamic,
            nullable,
        },
        (SchemaKind::Int, SchemaKind::Float) | (SchemaKind::Float, SchemaKind::Int) => Schema {
            kind: SchemaKind::Float,
            nullable,
        },
        (SchemaKind::Array(a_inner), SchemaKind::Array(b_inner)) => Schema {
            kind: SchemaKind::Array(Box::new(merge_schema(*a_inner, *b_inner))),
            nullable,
        },
        (SchemaKind::Object(a_map), SchemaKind::Object(b_map)) => Schema {
            kind: SchemaKind::Object(merge_object_maps(a_map, b_map)),
            nullable,
        },
        (SchemaKind::Bool, SchemaKind::Bool) => Schema {
            kind: SchemaKind::Bool,
            nullable,
        },
        (SchemaKind::Int, SchemaKind::Int) => Schema {
            kind: SchemaKind::Int,
            nullable,
        },
        (SchemaKind::Float, SchemaKind::Float) => Schema {
            kind: SchemaKind::Float,
            nullable,
        },
        (SchemaKind::String, SchemaKind::String) => Schema {
            kind: SchemaKind::String,
            nullable,
        },
        _ => Schema {
            kind: SchemaKind::Dynamic,
            nullable,
        },
    }
}

fn merge_object_maps(
    mut left: BTreeMap<String, Schema>,
    right: BTreeMap<String, Schema>,
) -> BTreeMap<String, Schema> {
    for (key, value) in right {
        if let Some(existing) = left.get_mut(&key) {
            let merged = merge_schema(existing.clone(), value);
            *existing = merged;
        } else {
            left.insert(key, value);
        }
    }
    left
}

#[derive(Clone, Copy, Debug)]
enum TargetLanguage {
    Go,
    Rust,
    TypeScript,
    Dart,
    Java,
    Php,
}

impl TargetLanguage {
    fn parse(input: &str) -> Option<Self> {
        match input.trim().to_ascii_lowercase().as_str() {
            "go" | "golang" => Some(Self::Go),
            "rust" => Some(Self::Rust),
            "typescript" | "ts" => Some(Self::TypeScript),
            "dart" | "flutter" => Some(Self::Dart),
            "java" => Some(Self::Java),
            "php" => Some(Self::Php),
            _ => None,
        }
    }
}

struct TypeNameRegistry {
    used: HashSet<String>,
    assigned: HashMap<String, String>,
}

impl TypeNameRegistry {
    fn new(root: &str) -> Self {
        let mut used = HashSet::new();
        used.insert(root.to_string());
        Self {
            used,
            assigned: HashMap::new(),
        }
    }

    fn name_for_field(
        &mut self,
        parent: &str,
        key: &str,
        in_array: bool,
        language: TargetLanguage,
    ) -> String {
        let id = format!("{}::{}::{}", parent, key, if in_array { "item" } else { "obj" });
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

struct TypeRenderer {
    language: TargetLanguage,
    registry: TypeNameRegistry,
    emitted: HashSet<String>,
    definitions: Vec<String>,
    uses_dynamic: bool,
    uses_structs: bool,
    uses_list: bool,
}

impl TypeRenderer {
    fn new(language: TargetLanguage, root_name: &str) -> Self {
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

    fn render(mut self, schema: &Schema, root_name: &str) -> String {
        let type_key = root_type_key(schema);
        match &schema.kind {
            SchemaKind::Object(_) => {
                self.emit_object(schema, root_name);
            }
            _ => {
                if let Some((object_schema, in_array)) = find_object_schema(schema, false) {
                    let nested_name = self
                        .registry
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
            let field_name = unique_name(
                sanitize_identifier(key, self.language),
                &mut used,
            );
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
            let field_name = unique_name(
                sanitize_identifier(key, self.language),
                &mut used,
            );
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
            let field_name = unique_name(
                sanitize_identifier(key, self.language),
                &mut used,
            );
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
            let field_name = unique_name(
                sanitize_identifier(key, self.language),
                &mut used,
            );
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
            let field_name = unique_name(
                sanitize_identifier(key, self.language),
                &mut used,
            );
            if field_name != *key {
                lines.push(format!("  // json: \"{}\"", escape_string(key)));
            }
            let field_type = self.php_type(schema, name, key, false);
            lines.push(format!("  public {} ${};", field_type, field_name));
        }
        lines.push("}".to_string());
        lines.join("\n")
    }

    fn go_type(&mut self, schema: &Schema, parent: &str, key: &str, in_array: bool) -> String {
        let base = match &schema.kind {
            SchemaKind::Bool => "bool".to_string(),
            SchemaKind::Int => "int64".to_string(),
            SchemaKind::Float => "float64".to_string(),
            SchemaKind::String => "string".to_string(),
            SchemaKind::Dynamic => "interface{}".to_string(),
            SchemaKind::Array(inner) => {
                let inner_type = self.go_type(inner, parent, key, true);
                format!("[]{}", inner_type)
            }
            SchemaKind::Object(_) => self
                .registry
                .name_for_field(parent, key, in_array, self.language),
        };

        if schema.nullable && base != "interface{}" {
            format!("*{}", base)
        } else {
            base
        }
    }

    fn rust_type(&mut self, schema: &Schema, parent: &str, key: &str, in_array: bool) -> String {
        let base = match &schema.kind {
            SchemaKind::Bool => "bool".to_string(),
            SchemaKind::Int => "i64".to_string(),
            SchemaKind::Float => "f64".to_string(),
            SchemaKind::String => "String".to_string(),
            SchemaKind::Dynamic => {
                self.uses_dynamic = true;
                "Value".to_string()
            }
            SchemaKind::Array(inner) => {
                let inner_type = self.rust_type(inner, parent, key, true);
                format!("Vec<{}>", inner_type)
            }
            SchemaKind::Object(_) => self
                .registry
                .name_for_field(parent, key, in_array, self.language),
        };

        if schema.nullable {
            format!("Option<{}>", base)
        } else {
            base
        }
    }

    fn ts_type(&mut self, schema: &Schema, parent: &str, key: &str, in_array: bool) -> String {
        let mut base = match &schema.kind {
            SchemaKind::Bool => "boolean".to_string(),
            SchemaKind::Int | SchemaKind::Float => "number".to_string(),
            SchemaKind::String => "string".to_string(),
            SchemaKind::Dynamic => "unknown".to_string(),
            SchemaKind::Array(inner) => {
                let inner_type = self.ts_type(inner, parent, key, true);
                let inner_type = if inner_type.contains(" | ") {
                    format!("({})", inner_type)
                } else {
                    inner_type
                };
                format!("{}[]", inner_type)
            }
            SchemaKind::Object(_) => self
                .registry
                .name_for_field(parent, key, in_array, self.language),
        };

        if schema.nullable {
            base = format!("{} | null", base);
        }
        base
    }

    fn dart_type(&mut self, schema: &Schema, parent: &str, key: &str, in_array: bool) -> String {
        let mut base = match &schema.kind {
            SchemaKind::Bool => "bool".to_string(),
            SchemaKind::Int => "int".to_string(),
            SchemaKind::Float => "double".to_string(),
            SchemaKind::String => "String".to_string(),
            SchemaKind::Dynamic => "dynamic".to_string(),
            SchemaKind::Array(inner) => {
                let inner_type = self.dart_type(inner, parent, key, true);
                format!("List<{}>", inner_type)
            }
            SchemaKind::Object(_) => self
                .registry
                .name_for_field(parent, key, in_array, self.language),
        };

        if schema.nullable && base != "dynamic" {
            base.push('?');
        }
        base
    }

    fn java_type(&mut self, schema: &Schema, parent: &str, key: &str, in_array: bool) -> String {
        let mut base = match &schema.kind {
            SchemaKind::Bool => {
                if in_array {
                    "Boolean".to_string()
                } else {
                    "boolean".to_string()
                }
            }
            SchemaKind::Int => {
                if in_array {
                    "Long".to_string()
                } else {
                    "long".to_string()
                }
            }
            SchemaKind::Float => {
                if in_array {
                    "Double".to_string()
                } else {
                    "double".to_string()
                }
            }
            SchemaKind::String => "String".to_string(),
            SchemaKind::Dynamic => "Object".to_string(),
            SchemaKind::Array(inner) => {
                self.uses_list = true;
                let inner_type = self.java_type(inner, parent, key, true);
                format!("List<{}>", inner_type)
            }
            SchemaKind::Object(_) => self
                .registry
                .name_for_field(parent, key, in_array, self.language),
        };

        if schema.nullable {
            base = box_java_primitive(&base);
        }
        base
    }

    fn php_type(&mut self, schema: &Schema, parent: &str, key: &str, in_array: bool) -> String {
        let base = match &schema.kind {
            SchemaKind::Bool => "bool".to_string(),
            SchemaKind::Int => "int".to_string(),
            SchemaKind::Float => "float".to_string(),
            SchemaKind::String => "string".to_string(),
            SchemaKind::Dynamic => "mixed".to_string(),
            SchemaKind::Array(inner) => {
                let _ = self.php_type(inner, parent, key, true);
                "array".to_string()
            }
            SchemaKind::Object(_) => self
                .registry
                .name_for_field(parent, key, in_array, self.language),
        };

        if schema.nullable && base != "mixed" {
            format!("?{}", base)
        } else {
            base
        }
    }
}

fn find_object_schema<'a>(schema: &'a Schema, in_array: bool) -> Option<(&'a Schema, bool)> {
    match &schema.kind {
        SchemaKind::Object(_) => Some((schema, in_array)),
        SchemaKind::Array(inner) => find_object_schema(inner, true),
        _ => None,
    }
}

fn root_type_key(schema: &Schema) -> &'static str {
    if matches!(&schema.kind, SchemaKind::Array(_)) {
        "item"
    } else {
        "value"
    }
}

fn sanitize_type_name(input: &str, language: TargetLanguage) -> String {
    let base = to_pascal_case(input);
    let base = if base.is_empty() { "Root".to_string() } else { base };
    let base = if starts_with_digit(&base) {
        format!("Type{}", base)
    } else {
        base
    };
    avoid_keyword(base, language)
}

fn sanitize_identifier(input: &str, language: TargetLanguage) -> String {
    let base = match language {
        TargetLanguage::Go => to_pascal_case(input),
        TargetLanguage::Rust => to_snake_case(input),
        TargetLanguage::TypeScript => to_lower_camel(input),
        TargetLanguage::Dart => to_lower_camel(input),
        TargetLanguage::Java => to_lower_camel(input),
        TargetLanguage::Php => to_lower_camel(input),
    };
    let base = if base.is_empty() { "field".to_string() } else { base };
    let base = if starts_with_digit(&base) {
        format!("field_{}", base)
    } else {
        base
    };
    avoid_keyword(base, language)
}

fn unique_name(base: String, used: &mut HashSet<String>) -> String {
    if !used.contains(&base) {
        used.insert(base.clone());
        return base;
    }
    let mut idx = 2;
    loop {
        let candidate = format!("{}{}", base, idx);
        if !used.contains(&candidate) {
            used.insert(candidate.clone());
            return candidate;
        }
        idx += 1;
    }
}

fn ts_property_name(input: &str) -> (String, bool) {
    if is_valid_ts_identifier(input) {
        (input.to_string(), false)
    } else {
        (input.to_string(), true)
    }
}

fn is_valid_ts_identifier(input: &str) -> bool {
    let mut chars = input.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || first == '_' || first == '$') {
        return false;
    }
    chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '$')
}

fn escape_string(input: &str) -> String {
    input
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
}

fn to_pascal_case(input: &str) -> String {
    let words = split_words(input);
    let mut out = String::new();
    for word in words {
        let mut chars = word.chars();
        if let Some(first) = chars.next() {
            out.push(first.to_ascii_uppercase());
            for ch in chars {
                out.push(ch.to_ascii_lowercase());
            }
        }
    }
    out
}

fn to_lower_camel(input: &str) -> String {
    let pascal = to_pascal_case(input);
    let mut chars = pascal.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };
    let mut out = String::new();
    out.push(first.to_ascii_lowercase());
    out.extend(chars);
    out
}

fn to_snake_case(input: &str) -> String {
    let words = split_words(input);
    words
        .into_iter()
        .map(|word| word.to_ascii_lowercase())
        .collect::<Vec<String>>()
        .join("_")
}

fn split_words(input: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut current = String::new();
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() {
            current.push(ch);
        } else if !current.is_empty() {
            words.push(current.clone());
            current.clear();
        }
    }
    if !current.is_empty() {
        words.push(current);
    }
    words
}

fn starts_with_digit(input: &str) -> bool {
    input.chars().next().map(|ch| ch.is_ascii_digit()).unwrap_or(false)
}

fn avoid_keyword(name: String, language: TargetLanguage) -> String {
    let keyword = match language {
        TargetLanguage::Go => is_go_keyword(&name),
        TargetLanguage::Rust => is_rust_keyword(&name),
        TargetLanguage::Dart => is_dart_keyword(&name),
        TargetLanguage::Java => is_java_keyword(&name),
        TargetLanguage::Php => is_php_keyword(&name),
        TargetLanguage::TypeScript => false,
    };
    if keyword {
        format!("{}_", name)
    } else {
        name
    }
}

fn is_go_keyword(name: &str) -> bool {
    matches!(
        name,
        "break" | "default" | "func" | "interface" | "select" | "case" | "defer" | "go" |
            "map" | "struct" | "chan" | "else" | "goto" | "package" | "switch" | "const" |
            "fallthrough" | "if" | "range" | "type" | "continue" | "for" | "import" |
            "return" | "var"
    )
}

fn is_rust_keyword(name: &str) -> bool {
    matches!(
        name,
        "as" | "break" | "const" | "continue" | "crate" | "else" | "enum" | "extern" |
            "false" | "fn" | "for" | "if" | "impl" | "in" | "let" | "loop" | "match" |
            "mod" | "move" | "mut" | "pub" | "ref" | "return" | "self" | "Self" |
            "static" | "struct" | "super" | "trait" | "true" | "type" | "unsafe" |
            "use" | "where" | "while" | "async" | "await" | "dyn"
    )
}

fn is_dart_keyword(name: &str) -> bool {
    matches!(
        name,
        "abstract" | "as" | "assert" | "async" | "await" | "break" | "case" | "catch" |
            "class" | "const" | "continue" | "covariant" | "default" | "deferred" | "do" |
            "dynamic" | "else" | "enum" | "export" | "extends" | "extension" | "external" |
            "factory" | "false" | "final" | "finally" | "for" | "Function" | "get" |
            "hide" | "if" | "implements" | "import" | "in" | "interface" | "is" |
            "late" | "library" | "mixin" | "new" | "null" | "on" | "operator" | "part" |
            "rethrow" | "return" | "set" | "show" | "static" | "super" | "switch" |
            "sync" | "this" | "throw" | "true" | "try" | "typedef" | "var" | "void" |
            "while" | "with" | "yield"
    )
}

fn is_php_keyword(name: &str) -> bool {
    matches!(
        name,
        "class" | "function" | "public" | "private" | "protected" | "static" | "abstract" |
            "final" | "extends" | "implements" | "interface" | "namespace" | "use" |
            "trait" | "const"
    )
}

fn is_java_keyword(name: &str) -> bool {
    matches!(
        name,
        "abstract" | "assert" | "boolean" | "break" | "byte" | "case" | "catch" | "char" |
            "class" | "const" | "continue" | "default" | "do" | "double" | "else" | "enum" |
            "extends" | "final" | "finally" | "float" | "for" | "goto" | "if" |
            "implements" | "import" | "instanceof" | "int" | "interface" | "long" |
            "native" | "new" | "package" | "private" | "protected" | "public" | "return" |
            "short" | "static" | "strictfp" | "super" | "switch" | "synchronized" | "this" |
            "throw" | "throws" | "transient" | "try" | "void" | "volatile" | "while" | "true" |
            "false" | "null" | "var" | "record" | "sealed" | "permits" | "yield"
    )
}

fn box_java_primitive(input: &str) -> String {
    match input {
        "boolean" => "Boolean".to_string(),
        "long" => "Long".to_string(),
        "double" => "Double".to_string(),
        _ => input.to_string(),
    }
}
