# Python adapter

The Python adapter implements the shared LanguageAdapter contract using the MIT
Tree-sitter Python grammar. Modules, functions, classes, methods, top-level simple
assignments, literal imports, local references, resolvable calls and test names
use the same graph, IDs, spans and evidence as JS/TS. No interpreter, import hooks,
virtual environment or package setup executes. Parse errors produce partial-index
diagnostics. The same file, tree, time and repository budgets apply.

Absolute imports search indexed root and src modules; relative imports use the
package directory. Both module.py and module/__init__.py are considered, and
ambiguity is reported. Imported aliases and package reexports resolve statically.
self.method calls resolve within the lexical class; arbitrary instance dispatch,
inheritance, monkey-patching, sys.path changes, namespace packages with no concrete
module, dynamic imports, star imports and decorators do not establish extra targets.
Parameters prevent name resolution to unrelated globals. Runtime rebinding and
local assignment shadowing remain limitations. test_ functions and Test classes
are marked, alongside test_/_test files and tests directories. No coverage claim
or Python framework adapter is provided in this release.
