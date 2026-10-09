# 0003 — Do not execute repository code

Accepted. Repository contents are untrusted input. Ordinary indexing must never
run project package scripts, executables, tests or build commands. Developer
commands in README build Threadmark itself, not repositories it analyzes.

