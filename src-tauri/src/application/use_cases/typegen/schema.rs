use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Clone, Debug)]
pub(super) struct Schema {
    pub(super) kind: SchemaKind,
    pub(super) nullable: bool,
}

#[derive(Clone, Debug)]
pub(super) enum SchemaKind {
    Bool,
    Int,
    Float,
    String,
    Array(Box<Schema>),
    Object(BTreeMap<String, Schema>),
    Dynamic,
}

pub(super) fn infer_schema(value: &Value) -> Schema {
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

pub(super) fn find_object_schema<'a>(
    schema: &'a Schema,
    in_array: bool,
) -> Option<(&'a Schema, bool)> {
    match &schema.kind {
        SchemaKind::Object(_) => Some((schema, in_array)),
        SchemaKind::Array(inner) => find_object_schema(inner, true),
        _ => None,
    }
}

pub(super) fn root_type_key(schema: &Schema) -> &'static str {
    if matches!(&schema.kind, SchemaKind::Array(_)) {
        "item"
    } else {
        "value"
    }
}
