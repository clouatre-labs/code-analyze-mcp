---
applyTo: "src/languages/**/*.rs"
excludeAgent: "coding-agent"
description: "tree-sitter query conventions and language handler patterns"
---

## LanguageInfo registration

New language handlers must register all fields in `src/languages/mod.rs` `get_language_info()`. Flag a new language file in `src/languages/` that is not matched by a new arm in the `get_language_info` match block.

Flag a new language extension that is not added to the map in `src/lang.rs`.

## tree-sitter query strings

- Queries are `&'static str` constants named `ELEMENT_QUERY`, `CALL_QUERY`, `REFERENCE_QUERY`, `IMPORT_QUERY`, `IMPL_QUERY`, `IMPL_TRAIT_QUERY`. Flag new query constants with non-standard names.
- Queries must use named captures (`@func_name`, `@call`, `@import_path`). Flag positional-only captures.
- `IMPL_TRAIT_QUERY` must only match `(type_identifier)` for the trait name -- scoped traits (`impl io::Sink for T`) are intentionally out of scope. Flag attempts to extend it to `scoped_type_identifier`.

## Handler function signatures

All four handler function types must match the type aliases in `src/languages/mod.rs`:

- `ExtractFunctionNameHandler`: `fn(&Node, &str, &str) -> Option<String>`
- `FindMethodForReceiverHandler`: `fn(&Node, &str, Option<usize>) -> Option<String>`
- `FindReceiverTypeHandler`: `fn(&Node, &str) -> Option<String>`
- `ExtractInheritanceHandler`: `fn(&Node, &str) -> Vec<String>`

Flag handler functions with signatures that deviate from these types.

## Optional fields

`reference_query`, `import_query`, `impl_query`, `impl_trait_query` are `Option<&'static str>`. Use `None` for languages that do not support the concept. Flag `Some("")` (empty string) as a substitute for `None`.
