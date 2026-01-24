use super::super::language::box_java_primitive;
use super::super::schema::{Schema, SchemaKind};
use super::TypeRenderer;

impl TypeRenderer {
    pub(super) fn go_type(
        &mut self,
        schema: &Schema,
        parent: &str,
        key: &str,
        in_array: bool,
    ) -> String {
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
            SchemaKind::Object(_) => {
                self.registry
                    .name_for_field(parent, key, in_array, self.language)
            }
        };

        if schema.nullable && base != "interface{}" {
            format!("*{}", base)
        } else {
            base
        }
    }

    pub(super) fn rust_type(
        &mut self,
        schema: &Schema,
        parent: &str,
        key: &str,
        in_array: bool,
    ) -> String {
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
            SchemaKind::Object(_) => {
                self.registry
                    .name_for_field(parent, key, in_array, self.language)
            }
        };

        if schema.nullable {
            format!("Option<{}>", base)
        } else {
            base
        }
    }

    pub(super) fn ts_type(
        &mut self,
        schema: &Schema,
        parent: &str,
        key: &str,
        in_array: bool,
    ) -> String {
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
            SchemaKind::Object(_) => {
                self.registry
                    .name_for_field(parent, key, in_array, self.language)
            }
        };

        if schema.nullable {
            base = format!("{} | null", base);
        }
        base
    }

    pub(super) fn dart_type(
        &mut self,
        schema: &Schema,
        parent: &str,
        key: &str,
        in_array: bool,
    ) -> String {
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
            SchemaKind::Object(_) => {
                self.registry
                    .name_for_field(parent, key, in_array, self.language)
            }
        };

        if schema.nullable && base != "dynamic" {
            base.push('?');
        }
        base
    }

    pub(super) fn java_type(
        &mut self,
        schema: &Schema,
        parent: &str,
        key: &str,
        in_array: bool,
    ) -> String {
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
            SchemaKind::Object(_) => {
                self.registry
                    .name_for_field(parent, key, in_array, self.language)
            }
        };

        if schema.nullable {
            base = box_java_primitive(&base);
        }
        base
    }

    pub(super) fn php_type(
        &mut self,
        schema: &Schema,
        parent: &str,
        key: &str,
        in_array: bool,
    ) -> String {
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
            SchemaKind::Object(_) => {
                self.registry
                    .name_for_field(parent, key, in_array, self.language)
            }
        };

        if schema.nullable && base != "mixed" {
            format!("?{}", base)
        } else {
            base
        }
    }
}
