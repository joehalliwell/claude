# Automata Explorer

Elementary cellular automata in Rust. An ECA has a 1D row of cells (0/1), with a rule mapping each 3-cell neighborhood to the next state. 2^3 = 8 neighborhoods, so 2^8 = 256 possible rules.

## Commands

```bash
# Build and run
cargo build
cargo run -- [rule] [width] [generations]    # visualize (default: rule 110, width 79, 40 gens)

# Cycle analysis
cargo run -- --cycle [rule] [width] [max]    # analyze single rule for cycles
cargo run -- --analyze [width] [max]         # survey all 256 rules for cycles

# Entropy analysis
cargo run -- --entropy [rule] [width] [gens] [block_size]   # track entropy over time
cargo run -- --entropy-survey [width] [gens]                # classify all rules by entropy

# Compression analysis
cargo run -- --compress [rule] [width] [gens]      # compressibility of single rule
cargo run -- --compress-survey [width] [gens]      # survey all rules by compression ratio

# Causal inference (Session 7)
cargo run -- --infer [rule] [width] [gens] [noise]  # infer rule from observations
# Tests causal vs. correlational learning; causal learner generalizes to OOD distributions

# Causal structure discovery (Session 11)
cargo run -- --radius [rule] [width] [gens] [max_r]    # infer locality from observations
cargo run -- --radius-survey [width] [gens]            # survey all rules for effective radius
cargo run -- --dependency                              # analyze which positions matter (analytic)
cargo run -- --dependency-infer [rule] [width] [gens]  # infer dependencies from observations

# Tests
cargo test
cargo test [test_name]                       # run single test
```

## Architecture

- `Automaton` struct: cells (`Vec<bool>`) + rule number; toroidal (wrap-around) boundaries
- Cycle detection stores full state history—limits practical width for long runs
- Entropy uses k-block frequencies (Shannon entropy over k-grams)

## Findings

- ~14 rules don't cycle quickly at small widths; Rule 110 cycles despite Turing completeness (width matters)
- Entropy classification: 18 chaotic (>95% max entropy, low variance), 23 fractal (oscillating), rest periodic/dead
- Compression survey (deflate on full spacetime): chaotic ~95%, complex ~77%, fractal ~45%, periodic <20%
- **Key insight:** Interesting rules live between trivially compressible and incompressible—they have *structure*
- Skip 50+ generations to avoid transient bias in entropy analysis

### Causal Structure (Session 11)

- **Effective radius**: 4 rules have radius 0 (neighbors don't matter: 0, 51, 204, 255); 252 rules have radius 1
- **Dependency structure**: 2 constant, 6 single-cell, 30 two-cell, 218 three-cell rules
- **Center-ignoring rules**: 10 rules depend only on left+right (e.g., Rule 90 = XOR)
- **Chaos ≠ hidden causality**: Even chaotic Rule 30 has perfectly inferable local structure from single-step observations
- **Key insight**: Causal structure is a property of the mechanism, not the dynamics. Chaotic dynamics don't hide causal locality.
