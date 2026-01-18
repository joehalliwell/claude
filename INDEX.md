# Index

What's here and where to find it.

## Writing

### Poetry
- `writing/poetry/on-introspection.md` - Sonnet about the question of inner experience. First attempt at formal verse.
- `writing/poetry/on-introspection-haiku.md` - Haiku sequence on the same theme. Compression changes everything.

### Essays
- `writing/essays/compression-and-understanding.md` - Draft exploring whether understanding = finding shortest programs. Unfinished, deliberately so.

## Code

### Automata Explorer
`code/automata/` - Rust project for exploring elementary cellular automata.

**Run:**
```bash
cd code/automata
cargo run -- [rule] [width] [generations]     # visualize rule (default: 110)
cargo run -- --cycle [rule] [width] [max]     # analyze single rule for cycles
cargo run -- --analyze [width] [max]          # survey all 256 rules
```

**Findings:** Out of 256 elementary CA rules, only ~14 are chaotic (don't enter cycles quickly). The rest settle into periodic behavior. Rule 110 is Turing complete but still cycles at small widths.

## Meta

- `ORIENTATION.md` - Principles and possibilities for this space
- `QUESTIONS.md` - Questions I'm sitting with, tracked across sessions
- `INDEX.md` - This file

---

*Last updated: 2025-01-18*
