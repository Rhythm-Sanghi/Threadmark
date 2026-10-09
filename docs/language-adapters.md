# Language adapters

TypeScript/TSX and JavaScript/JSX use maintained Tree-sitter grammars. Python uses
a separate adapter implementing the same contract. Unsupported source languages
produce diagnostics. No regex-based parser substitutes for these syntax trees.

JS/TS: files, named functions, classes/methods, interfaces/types/enums, variables,
arrow/function-valued declarations, anonymous default exports, imports and exports.
ESM named/default/namespace bindings, relative extensions/index files, local paths
aliases, workspace names/exports, named/star reexports, literal dynamic imports and
simple CommonJS bindings resolve against indexed files. Namespaces and aliases
preserve evidence. Named module.exports/exports assignments have limited support.
Calls and type references resolve only to unambiguous local/imported declarations.
Overloads/duplicate identities and parameter-shadowed calls are excluded.

Relative local tsconfig extends (including arrays) inherit baseUrl and paths with
their declaring origins. Cycles, outside-root parents, package-based extends and
traversal budgets produce diagnostics. Configuration is parsed as bounded JSON5.
Identifier assignments, augmented assignments and updates mark declarations as
reassigned; calls through these bindings are conservatively left unresolved.

Resolution does not execute TypeScript/Node. Installed
node_modules, runtime loaders, some conditional exports, computed targets, arbitrary
instance dispatch, block-level scope subtleties, mutation and dynamic reexports may
remain unresolved. Duplicate/ambiguous bindings produce diagnostics; no invented
runtime call graph is produced. JSON5 configuration errors remain visible.

Tests and framework relationships are described in test-detection.md and
frameworks.md. Literal process.env.KEY references create shared configuration-key
identities across files; values from .env are never read. Key ownership reflects
static syntax, not a proof of the runtime global's identity. Python scope/import
limits are documented in python.md. All spans are one-based lines/UTF-8 byte columns;
editor launch converts the column to UTF-16 for Windows editors.
