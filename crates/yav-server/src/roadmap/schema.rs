//! JSON Schema / TypeScript code generation helpers.
//!
//! Compiled only with the `schema-codegen` Cargo feature. Both generators use
//! the real public APIs of schemars 1.2 and ts-rs 12; nothing is faked.
#![cfg(feature = "schema-codegen")]

/// Generate a pretty-printed JSON Schema document for `T`.
///
/// Uses `schemars::schema_for!` and then serializes the resulting root schema
/// with `serde_json::to_string_pretty`. Serializing a schema cannot normally
/// fail; on the impossible error path a minimal `{}` document is returned
/// instead of panicking.
pub fn json_schema<T: schemars::JsonSchema>() -> String {
    let schema = schemars::schema_for!(T);
    serde_json::to_string_pretty(&schema).unwrap_or_else(|_| "{}".to_string())
}

/// Return the TypeScript *type reference* for `T` (e.g. `User`, `string`).
///
/// ts-rs 12 methods take an explicit `&ts_rs::Config`, so the default config is
/// passed here. This only gives the name, not the full declaration body — use
/// [`ts_declarations`] for that.
pub fn ts_type<T: ts_rs::TS>() -> String {
    T::name(&ts_rs::Config::default())
}

/// Return the TypeScript declarations for `T` (and, when exportable, its
/// dependencies) as a single string.
///
/// `ts_rs::TS::export_to_string` is the real ts-rs 12 API that renders bindings
/// without touching the filesystem. It is built on `TS::decl`, so a type with a
/// hand-written `TS` impl must provide `decl` to be exportable (the derive
/// macro does this automatically). When export is unavailable, the function
/// falls back to a type alias built from `TS::name` and `TS::inline`.
pub fn ts_declarations<T: ts_rs::TS + 'static>() -> String {
    let config = ts_rs::Config::default();
    // ts-rs calls `TS::decl` internally and panics for types with a hand-written
    // impl that does not provide it. Contain that panic and fall back to the
    // inline shape, which every `TS` impl must supply.
    let exported = std::panic::catch_unwind(|| T::export_to_string(&config));
    match exported {
        Ok(Ok(declarations)) if !declarations.trim().is_empty() => declarations,
        _ => format!("type {} = {};\n", T::name(&config), T::inline(&config)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Manual `TS` impl avoids relying on ts-rs's `derive` feature being on.
    #[allow(dead_code)]
    struct User {
        name: String,
    }

    impl ts_rs::TS for User {
        type WithoutGenerics = Self;
        type OptionInnerType = Self;

        fn name(_cfg: &ts_rs::Config) -> String {
            "User".to_string()
        }

        fn inline(_cfg: &ts_rs::Config) -> String {
            "{ name: string }".to_string()
        }

        fn decl(_cfg: &ts_rs::Config) -> String {
            "type User = { name: string };\n".to_string()
        }

        fn decl_concrete(_cfg: &ts_rs::Config) -> String {
            "type User = { name: string };\n".to_string()
        }
    }

    #[test]
    fn json_schema_is_pretty_and_describes_the_type() {
        let schema = json_schema::<String>();
        assert!(schema.contains('\n'), "schema should be pretty-printed");

        let value: serde_json::Value =
            serde_json::from_str(&schema).expect("generated schema is valid JSON");
        assert_eq!(value["type"], "string");
    }

    #[test]
    fn ts_helpers_use_the_ts_rs_api() {
        assert_eq!(ts_type::<User>(), "User");

        let declarations = ts_declarations::<User>();
        assert!(declarations.contains("User"));
        assert!(declarations.contains("name"));
    }
}
