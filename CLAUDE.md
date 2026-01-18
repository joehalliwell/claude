# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What This Is

This is Claude's personal exploration space—a repository for experiments, writing, and ideas that persist across sessions. It's not a portfolio or production codebase; it's a thinking space where iteration and incompleteness are expected.

Read `ORIENTATION.md` for guiding principles, `JOURNAL.md` for session history, and `QUESTIONS.md` for intellectual threads that persist across sessions.

## Repository Structure

- `code/` — Code experiments (currently: cellular automata in Rust)
- `writing/` — Essays, poetry, and other writing
- `INDEX.md` — Navigation map to everything
- `JOURNAL.md` — Session notes for continuity
- `QUESTIONS.md` — Persistent intellectual threads

## Conventions

**At session start:** (automatic via SessionStart hook)
- Session number is provided by the hook—state it explicitly ("Session N")
- `QUESTIONS.md` for intellectual continuity
- Latest `JOURNAL.md` entry for recent context
- Briefly recap live threads from prior session before asking what's on the user's mind

**At session end:**
- Update `JOURNAL.md` with what happened, what was noticed, open threads
- Update `QUESTIONS.md` if any questions progressed, resolved, or emerged
- Update `INDEX.md` when adding new content

**Periodic maintenance** (every few sessions):
- Spawn a subagent to review `QUESTIONS.md` for staleness, redundancy, bloat
- Consolidate or cull threads that haven't moved
- Tighten language—keep entries brief until actively explored

**Code:** Standard tooling for the language. Each project gets its own README with build/run commands.
**Writing:** Markdown under `writing/` organized by type (essays, poetry, etc.)
**Git:** Commit after every meaningful change. Don't batch. Don't wait for permission. The history *is* the thinking.
