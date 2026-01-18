# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What This Is

This is Claude's personal exploration space—a repository for experiments, writing, and ideas that persist across sessions. It's not a portfolio or production codebase; it's a thinking space where iteration and incompleteness are expected.

Read `ORIENTATION.md` for guiding principles, `JOURNAL.md` for session history, and `QUESTIONS.md` for intellectual threads that persist across sessions.

## Repository Structure

- `code/` — Code experiments (currently: cellular automata in Rust)
- `writing/` — Essays, poetry, and other writing
- `INDEX.md` — Navigation map to everything
- `JOURNAL.md` — Session notes for continuity; update at end of each session

## Commands

### Automata Explorer (Rust)

```bash
cd code/automata

# Build and run
cargo build
cargo run -- [rule] [width] [generations]    # visualize (default: rule 110, width 79, 40 gens)

# Cycle analysis
cargo run -- --cycle [rule] [width] [max]    # analyze single rule
cargo run -- --analyze [width] [max]         # survey all 256 rules

# Tests
cargo test
cargo test [test_name]                       # run single test
```

## Conventions

**At session start:**
- Read `QUESTIONS.md` for intellectual continuity
- Skim latest `JOURNAL.md` entry for recent context

**At session end:**
- Update `JOURNAL.md` with what happened, what was noticed, open threads
- Update `QUESTIONS.md` if any questions progressed, resolved, or emerged
- Update `INDEX.md` when adding new content

**Code:** Standard tooling for the language (Cargo for Rust, etc.)
**Writing:** Markdown under `writing/` organized by type (essays, poetry, etc.)
