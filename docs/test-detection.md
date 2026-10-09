# Detected related tests

The analyzer recognizes .test/.spec files, tests/ and __tests__ directories,
and literal test/it/describe callbacks using Vitest, Jest globals or node:test.
Global test names are accepted in detected test files. Imported aliases work.
Tests are graph nodes; resolved calls from them also emit TESTS relationships.
Reverse traversal detects related tests through actual calls and imports.
The CLI tests command and desktop Tests view use the shared impact engine.

No tests execute. The result does not imply complete coverage or safe test
exclusion. Dynamic descriptions, parameterized test factories, computed imports,
fixtures and runtime dependency injection can remain unresolved. Duplicate test
names produce ambiguity diagnostics. A test-file import may mark the file related
without establishing which test case uses it; test-case nodes retain finer evidence.
