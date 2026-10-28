% S-EXPR-CORE-1.0
% Normative contract for the s-expr-core module

# 1. Introduction

This document is a normative contract for the s-expr-core module.

It defines a minimal yet precise standard for:

- S-expression syntax
- The in-memory data model
- The error model
- Normalization and pretty-printing
- The minimal required API surface

The intent is that multiple independent implementations (Rust, other languages, alternative runtimes) can implement this contract and remain interoperable at the S-expression level.

This document uses the key words MUST, MUST NOT, REQUIRED, SHALL, SHALL NOT, SHOULD, SHOULD NOT, RECOMMENDED, MAY, and OPTIONAL as described in RFC 2119.

# 2. Scope and Non-goals

## 2.1 Scope

An implementation that claims conformance to this contract:

- MUST implement a deterministic parser for the S-expression syntax defined in section 4.
- MUST expose the data model defined in section 5 (Expr, AtomKind, Node, Span) or an isomorphic representation.
- MUST expose the error model defined in section 7.
- MUST provide the API surface defined in section 6.
- MUST implement canonical pretty-printing and roundtrip invariants as defined in section 8.
- MUST support dialect configuration and resource limits as defined in section 9.

The contract is independent of:

- Any specific query language (ZeroTree, LVS, or others)
- Any execution engine, bytecode format, or VM
- Any storage or persistence model

## 2.2 Non-goals

This contract explicitly does not specify:

- Any query semantics, bytecode, or VM behavior.
- Any domain-specific constructs such as captures, fields, or predicates.
- Any derive macros or automatic mapping between S-expressions and host-language types.
- Any persistence or serialization format other than the S-expression text syntax.

Higher-level modules (for example, ZeroTree query, LVS S-expression frontends, or other DSLs) MUST define their own ASTs and semantics separately, using this module only as a syntactic and structural foundation.

# 3. Terminology and Conformance

## 3.1 Terminology

- Implementation: a concrete library or module that claims to implement this contract.
- Consumer: any code that calls the implementation’s API.
- Default dialect: the dialect configuration defined by the implementation when the consumer does not override dialect settings.
- Node: a single element in the parsed S-expression tree (list or atom) with an associated span.
- Span: a source location descriptor that identifies a contiguous region of the input.

## 3.2 Conformance

An implementation conforms to this contract if and only if it satisfies all MUST and MUST NOT requirements in this document for:

- Syntax acceptance and rejection (section 4)
- Data model invariants (section 5)
- API behavior (section 6)
- Error model (section 7)
- Normalization and roundtrip behavior (section 8)
- Dialect and limit semantics (section 9)
- Versioning rules (section 10)

Consumers MAY rely on any MUST / MUST NOT guarantees as stable behavior across minor and patch versions of a conforming implementation.

# 4. Syntax

## 4.1 Base syntax

A conforming implementation under the default dialect:

- MUST accept the following constructs:
  - Lists delimited by left and right parentheses.
  - Atoms separated by whitespace.
  - Line comments starting with a single semicolon character and extending to the end of the line.
- MUST support arbitrarily nested lists, subject only to explicit resource limits (see section 9).
- MUST treat any sequence of characters that is not part of a string, list delimiter, or comment as candidate atom text, subject to the atom rules below.

The default dialect’s atom categories:

- Symbols
- Strings
- Integers
- Booleans
- Keywords

Optional dialect extensions (additional numeric formats, additional atom categories, quoting, etc.) MAY be supported but:

- MUST be disabled by default unless explicitly specified in a later revision of this contract.
- MUST NOT change the set of texts accepted by the default dialect defined here.

## 4.2 Informal grammar (default dialect)

The default dialect MUST accept exactly the language defined by the following grammar, when interpreted over Unicode scalar values:

```text
program    ::= ws* form* ws*
form       ::= list | atom

list       ::= "(" ws* form* ws* ")"

atom       ::= symbol | string | integer | boolean | keyword

symbol     ::= symbol-start symbol-char*
symbol-start ::= any character except whitespace, "(", ")", ";", or string delimiter
symbol-char  ::= symbol-start | digit | other non-whitespace characters

string     ::= "\"" string-char* "\""
string-char ::= any character except "\"" or "\\" |
                "\\" escape-seq
escape-seq ::= "\"" | "\\" | "n" | "r" | "t"

integer    ::= ["-"] digit+
boolean    ::= "true" | "false"
keyword    ::= ":" symbol

comment    ::= ";" [^"\n"]* ("\n" | end-of-input)

ws         ::= " " | "\t" | "\r" | "\n" | comment
digit      ::= "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9"
```

An implementation MAY internally represent or recognize additional escape sequences or numeric forms, but:

- It MUST reject any input that is not accepted by the grammar above when operating in the default dialect.
- It MUST document any dialect options that extend the accepted language beyond this grammar, and such options MUST be disabled by default.

# 5. Data Model

## 5.1 Logical types

An implementation MUST expose logical types equivalent to the following:

- Expr
  - List: ordered sequence of Node values.
  - Atom: a single AtomKind value.
- AtomKind
  - Symbol
  - String
  - Integer
  - Boolean
  - Keyword
  - OPTIONAL additional variants (for example, other numeric types) that do not change the semantics of existing atoms.
- Node
  - expr: an Expr value (list or atom).
  - span: a Span value.
- Span
  - A half-open byte range [start, end) into the original input.
  - Line and column coordinates for the start and end positions.

Implementations MAY use different concrete type names and layouts, but they:

- MUST be able to represent these logical variants without loss of information.
- MUST associate a Span with every Node.

## 5.2 Invariants

A conforming implementation MUST enforce the following invariants:

- Every Node produced by a successful parse MUST have a non-empty Span.
- Spans for different Nodes MAY overlap but MUST always refer to valid byte ranges within the input.
- For any given input, dialect, and limit configuration, parsing MUST be deterministic:
  - Repeated calls with identical parameters MUST produce structurally equivalent trees or identical error codes and spans.
- The order of child Nodes in a list MUST correspond exactly to their order in the input.

# 6. API Surface

This section specifies the required logical API. Implementations MAY expose additional helpers, but they MUST NOT weaken the guarantees for the functions defined here.

## 6.1 Required functions (logical)

An implementation MUST provide functions with the following behavior:

- parse
  - Input:
    - A string of source text.
    - OPTIONAL dialect configuration; if omitted, the default dialect MUST be used.
  - Output:
    - On success: an ordered sequence of top-level Node values.
    - On error: a structured parse error as defined in section 7.
  - Requirements:
    - The function MUST NOT panic on malformed input.
    - The function MUST return either a complete tree for all top-level forms or a single hard error; it MUST NOT return partially constructed trees without an accompanying error.
    - The function MUST report the first hard error encountered, with a precise Span.

Implementations MAY realize this logical parse operation as multiple entry points (for example, a parse that uses the default dialect and a parse_with_dialect that accepts an explicit DialectConfig), provided that the combined behavior satisfies the requirements above.

- parse_with_limit
  - Input:
    - A string of source text.
    - A dialect configuration.
    - A set of resource limits (see section 9).
  - Output:
    - On success: an ordered sequence of top-level Node values.
    - On error: a structured parse error, including resource limit violations.
  - Requirements:
    - The function MUST detect and report violations of configured limits.
    - Limit violations MUST be surfaced using the error model (section 7) and MUST include a Span that best approximates the location of the violating construct.

Implementations MAY provide additional variants, such as:

- parse_with_dialect (input, dialect)
- Iterator-based APIs for streaming consumption of top-level forms

Such additions MUST be defined in terms of the same data model and error model.

## 6.2 Rust reference API (normative for the Rust crate)

The Rust reference implementation of this contract (the s-expr-core crate) MUST expose at least the following public functions and types:

- Types:
  - Span
  - Position
  - AtomKind
  - Expr
  - Node
  - DialectConfig
  - Limits
  - ParseError
  - ParseErrorCode
  - ErrorDetails
- Functions:
  - parse(input: &str) -> Result<Vec<Node>, ParseError>
  - parse_with_dialect(input: &str, dialect: &DialectConfig) -> Result<Vec<Node>, ParseError>
  - parse_with_limit(input: &str, dialect: &DialectConfig, limits: &Limits) -> Result<Vec<Node>, ParseError>
  - pretty_print(nodes: &[Node]) -> String

Iterator utilities such as ExprIter and AtomIter are RECOMMENDED for ergonomics but not REQUIRED by this contract.

# 7. Error Model

## 7.1 Structure

Parsing errors MUST be structured and machine-readable. At minimum, each hard error MUST include:

- Error code
  - A short, stable identifier such as S_EXPR_UNBALANCED_PAREN or S_EXPR_INVALID_ATOM.
- Message
  - A human-readable description suitable for diagnostics and logs.
- Span
  - The location of the error in the input, expressed as a Span.
- Details
  - OPTIONAL structured payload with additional fields such as:
    - Expected token kinds.
    - Found token text.
    - Limit name and configured value in case of limit violations.

## 7.2 Guarantees

An implementation MUST guarantee that:

- Invalid input MUST NOT silently produce a successful parse result.
- If any hard error is detected, the parser MUST return an error and MUST NOT return an apparently complete tree without that error.
- Non-fatal diagnostics (for example, warnings) MUST be clearly distinguished from hard errors if exposed at all.

The set of error codes and their meanings MUST be documented by the implementation and MUST remain stable across minor and patch versions.

# 8. Normalization and Pretty-printing

## 8.1 Canonical representation

A conforming implementation MUST provide a function that produces a canonical textual representation of any valid S-expression tree that it can parse. In the Rust reference implementation this function is named pretty_print.

The canonical representation MUST satisfy the following properties:

- Whitespace between atoms MUST be deterministic.
- Line breaks and indentation MUST follow a consistent and documented strategy.
- String escaping rules MUST be consistent and MUST produce valid strings under the grammar in section 4.

## 8.2 Roundtrip invariant

For any text input src that is successfully parsed under a given dialect and limit configuration:

- Let nodes = parse(src, dialect) (ignoring limits for simplicity).
- Let out = pretty_print(nodes).
- Let nodes2 = parse(out, dialect).

Then:

- nodes2 MUST be structurally equivalent to nodes modulo:
  - Whitespace differences that do not affect atom boundaries.
  - Equivalent string escape sequences that decode to the same underlying text.

This requirement is normative: a conforming implementation MUST treat roundtrip preservation of the logical tree as a hard invariant.

## 8.3 Trivia

Comments are interleaved into a parse as trivia:

- `parse_with_trivia` MUST return the same top-level nodes as `parse_with_dialect`, plus every comment encountered in source order as a `Comment` value.
- A `Comment` MUST carry the comment body without its prefix character, with a trailing `\r` trimmed, plus its `Span`.
- `pretty_print_with_comments(nodes, comments)` MUST equal `pretty_print(nodes)` when `comments` is empty.
- With comments present, each comment MUST be emitted on its own line as `;` followed by its text, before the node that follows it in the source, at that node's indentation.
- Comments after the last element of a list MUST stay inside the list before `)`; comments after the last top-level node MUST be emitted at the end.
- A comment on the same line after a node MAY move to its own line.
- The roundtrip invariant MUST also hold for trivia: reparsing the pretty-printed output preserves the node structure (modulo spans) and the comment texts in the same order.

# 9. Dialect Configuration and Resource Limits

## 9.1 Dialect configuration

An implementation MUST provide a configuration object (referred to here as DialectConfig) that controls:

- Comments
  - Whether comments are enabled.
  - The single-character prefix that starts a comment (default: semicolon).
- Atom categories
  - Whether booleans are recognized as a distinct AtomKind.
  - Whether keywords are recognized as a distinct AtomKind.
  - Whether additional numeric formats are accepted.

The default DialectConfig MUST:

- Accept exactly the language defined in section 4.
- Remain backward compatible across minor and patch versions.

Any change to the default dialect that alters accepted or rejected inputs is a breaking change (see section 10).

## 9.2 Resource limits

An implementation MUST provide a configuration object (referred to here as Limits) that supports at least:

- Maximum nesting depth.
- Maximum list length.
- OPTIONAL global maximum node count.

If a limit is exceeded, the parser:

- MUST stop parsing.
- MUST return a structured error that identifies the violated limit.
- SHOULD provide a Span pointing at, or near, the construct that caused the violation.

Default numeric values for these limits are implementation-specific and MAY differ between conforming implementations; this contract constrains only the semantics of limits, not their concrete thresholds.

An implementation MAY embed Limits inside DialectConfig (for example, via a limits field) or keep them as separate configuration objects, as long as the same controls are available to consumers.

# 10. Compatibility and Evolution

## 10.1 Backward-compatible changes

The following changes are considered backward-compatible for a conforming implementation:

- Adding new AtomKind variants that do not change the meaning of existing inputs.
- Adding new optional dialect configuration fields that default to preserving existing behavior.
- Extending ErrorDetails with additional fields while keeping existing fields stable.
- Adding new helper APIs that are purely additive and do not change the semantics of existing functions.

## 10.2 Breaking changes

The following changes are considered breaking and MUST only occur across a major version boundary:

- Any change to the grammar in section 4 that alters the set of accepted inputs under the default dialect.
- Any change to the default dialect that changes how existing inputs are tokenized or parsed.
- Any change that removes or materially alters:
  - The required types in section 5.
  - The required functions and their behavior in section 6.
  - The error structure in section 7.
  - The roundtrip invariant in section 8.

Higher-level consumers must explicitly declare which major versions of this contract and its reference implementation they support.

# 11. Integration Guidance (Informative)

This section is informative and does not add new normative requirements.

- Generic query or language frontends:
  - Use the core parser to obtain an S-expression tree (a sequence of Node values).
  - Implement a separate module that maps Expr and AtomKind into a strongly typed AST.
  - Propagate Span values into diagnostics and query planning errors.
- Other tools:
  - Use the core parser for any S-expression based DSL or configuration format.

Consumers SHOULD avoid embedding domain-specific semantics into the S-expression core itself and SHOULD instead treat it strictly as a reusable frontend and normalization layer.

