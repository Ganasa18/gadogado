#[derive(Clone, Copy, Debug)]
pub(super) enum TargetLanguage {
    Go,
    Rust,
    TypeScript,
    Dart,
    Java,
    Php,
}

impl TargetLanguage {
    pub(super) fn parse(input: &str) -> Option<Self> {
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

pub(super) fn avoid_keyword(name: String, language: TargetLanguage) -> String {
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
        "break"
            | "default"
            | "func"
            | "interface"
            | "select"
            | "case"
            | "defer"
            | "go"
            | "map"
            | "struct"
            | "chan"
            | "else"
            | "goto"
            | "package"
            | "switch"
            | "const"
            | "fallthrough"
            | "if"
            | "range"
            | "type"
            | "continue"
            | "for"
            | "import"
            | "return"
            | "var"
    )
}

fn is_rust_keyword(name: &str) -> bool {
    matches!(
        name,
        "as" | "break"
            | "const"
            | "continue"
            | "crate"
            | "else"
            | "enum"
            | "extern"
            | "false"
            | "fn"
            | "for"
            | "if"
            | "impl"
            | "in"
            | "let"
            | "loop"
            | "match"
            | "mod"
            | "move"
            | "mut"
            | "pub"
            | "ref"
            | "return"
            | "self"
            | "Self"
            | "static"
            | "struct"
            | "super"
            | "trait"
            | "true"
            | "type"
            | "unsafe"
            | "use"
            | "where"
            | "while"
            | "async"
            | "await"
            | "dyn"
    )
}

fn is_dart_keyword(name: &str) -> bool {
    matches!(
        name,
        "abstract"
            | "as"
            | "assert"
            | "async"
            | "await"
            | "break"
            | "case"
            | "catch"
            | "class"
            | "const"
            | "continue"
            | "covariant"
            | "default"
            | "deferred"
            | "do"
            | "dynamic"
            | "else"
            | "enum"
            | "export"
            | "extends"
            | "extension"
            | "external"
            | "factory"
            | "false"
            | "final"
            | "finally"
            | "for"
            | "Function"
            | "get"
            | "hide"
            | "if"
            | "implements"
            | "import"
            | "in"
            | "interface"
            | "is"
            | "late"
            | "library"
            | "mixin"
            | "new"
            | "null"
            | "on"
            | "operator"
            | "part"
            | "rethrow"
            | "return"
            | "set"
            | "show"
            | "static"
            | "super"
            | "switch"
            | "sync"
            | "this"
            | "throw"
            | "true"
            | "try"
            | "typedef"
            | "var"
            | "void"
            | "while"
            | "with"
            | "yield"
    )
}

fn is_php_keyword(name: &str) -> bool {
    matches!(
        name,
        "class"
            | "function"
            | "public"
            | "private"
            | "protected"
            | "static"
            | "abstract"
            | "final"
            | "extends"
            | "implements"
            | "interface"
            | "namespace"
            | "use"
            | "trait"
            | "const"
    )
}

fn is_java_keyword(name: &str) -> bool {
    matches!(
        name,
        "abstract"
            | "assert"
            | "boolean"
            | "break"
            | "byte"
            | "case"
            | "catch"
            | "char"
            | "class"
            | "const"
            | "continue"
            | "default"
            | "do"
            | "double"
            | "else"
            | "enum"
            | "extends"
            | "final"
            | "finally"
            | "float"
            | "for"
            | "goto"
            | "if"
            | "implements"
            | "import"
            | "instanceof"
            | "int"
            | "interface"
            | "long"
            | "native"
            | "new"
            | "package"
            | "private"
            | "protected"
            | "public"
            | "return"
            | "short"
            | "static"
            | "strictfp"
            | "super"
            | "switch"
            | "synchronized"
            | "this"
            | "throw"
            | "throws"
            | "transient"
            | "try"
            | "void"
            | "volatile"
            | "while"
            | "true"
            | "false"
            | "null"
            | "var"
            | "record"
            | "sealed"
            | "permits"
            | "yield"
    )
}

pub(super) fn box_java_primitive(input: &str) -> String {
    match input {
        "boolean" => "Boolean".to_string(),
        "long" => "Long".to_string(),
        "double" => "Double".to_string(),
        _ => input.to_string(),
    }
}
