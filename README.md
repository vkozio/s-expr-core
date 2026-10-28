# s-expr-core

Small Rust library for parsing, normalizing, and pretty-printing S-expressions with spans and structured errors. For query engines, compilers, and S-expression tooling.

## Usage

**Git dependency:**

```toml
[dependencies]
s-expr-core = { git = "https://github.com/vkozio/s-expr-core.git", branch = "main" }
```

**crates.io (when published):**

```toml
[dependencies]
s-expr-core = "0.1"
```

**API:** Types `Span`, `Position`, `AtomKind`, `Expr`, `Node`, `DialectConfig`, `Limits`, `ParseError`, `ParseErrorCode`, `ErrorDetails`. Functions: `parse`, `parse_with_dialect`, `parse_with_limit`, `pretty_print`, `node_at`. Iterators: `ExprIter`, `AtomIter`. Helpers: `LineIndex`. See [CONTRACT.md](CONTRACT.md) for the full contract and grammar.

**Changelog:** v0.2.0 adds the `AtomKind::Float` atom (lexed from literals like `1.5`, `-0.25`, `1e9`, `2.5E-3`; gated by `DialectConfig::enable_floats`, default on), plus `node_at` and `LineIndex`.

## Example```rust
use s_expr_core::{parse, pretty_print, AtomKind, ExprIter};

let nodes = parse("(foo 1 (true :kw))").expect("parse ok");
println!("{}", pretty_print(&nodes));

for atom in ExprIter::new(&nodes).flat_map(|n| match &n.expr {
    s_expr_core::Expr::Atom(a) => Some(a),
    _ => None,
}) {
    if let AtomKind::Symbol(s) = atom { println!("{s}"); }
}
```

Errors: `ParseError` has `code` (`ParseErrorCode`), `message`, `span`, and optional `details`. Handle `UnbalancedParen`, `UnterminatedString`, `InvalidAtom`, `LimitExceeded(_)`, `UnexpectedEof`.

## Development

- **Check:** `cargo fmt --all` then `cargo clippy --all-targets --all-features -- -D warnings` then `cargo test --all`
- **Coverage:** `cargo install cargo-llvm-cov` then `cargo llvm-cov --all-features --workspace` (CI: soft 75% threshold)
- **Fuzz:** `cargo fuzz run fuzz_parse_never_panics`, `cargo fuzz run fuzz_roundtrip` (targets in `fuzz/`)
- **Bench:** `cargo bench --bench parse_pretty`

Optional npm scripts (from repo root): `check:rust`, `cov:rust`, `bench:rust` — see `package.json`.

## License

Apache-2.0
