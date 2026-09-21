---
"baxe": patch
"baxe-derive": patch
---

Reduce macro expansion overhead by reusing field bindings, preallocating output
collections, and parsing attribute options directly from tokens. Generate direct
`write!` calls instead of allocating intermediate formatted strings.

Correctly parse `logMessageWith` with or without spaces around `=` and report
malformed macro inputs as compiler diagnostics. The documented macro syntax and
public API remain unchanged.
