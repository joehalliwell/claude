/// Elementary Cellular Automata Explorer
///
/// An elementary CA has:
/// - A 1D row of cells, each 0 or 1
/// - A rule that maps each 3-cell neighborhood to the next state of the center cell
/// - 2^3 = 8 possible neighborhoods, so 2^8 = 256 possible rules
///
/// The rule number encodes the output for each neighborhood:
///   neighborhood:  111 110 101 100 011 010 001 000
///   bit position:   7   6   5   4   3   2   1   0
///
/// Example: Rule 110
///   110 = 0b01101110
///   111->0, 110->1, 101->1, 100->0, 011->1, 010->1, 001->1, 000->0

use std::collections::HashSet;
use std::fmt;
use std::hash::{Hash, Hasher};

#[derive(Clone, Eq, PartialEq)]
struct Automaton {
    cells: Vec<bool>,
    rule: u8,
}

impl Automaton {
    fn new(width: usize, rule: u8) -> Self {
        let mut cells = vec![false; width];
        // Start with single cell in center
        cells[width / 2] = true;
        Self { cells, rule }
    }

    fn from_cells(cells: Vec<bool>, rule: u8) -> Self {
        Self { cells, rule }
    }

    /// Apply rule to get next generation
    fn step(&mut self) {
        let n = self.cells.len();
        let mut next = vec![false; n];

        for i in 0..n {
            // Get neighborhood (wrapping at edges)
            let left = self.cells[(i + n - 1) % n];
            let center = self.cells[i];
            let right = self.cells[(i + 1) % n];

            // Convert neighborhood to index (0-7)
            let index = (left as u8) << 2 | (center as u8) << 1 | (right as u8);

            // Look up result in rule
            next[i] = (self.rule >> index) & 1 == 1;
        }

        self.cells = next;
    }

    fn width(&self) -> usize {
        self.cells.len()
    }

    /// Count live cells
    fn population(&self) -> usize {
        self.cells.iter().filter(|&&c| c).count()
    }

    /// Density as fraction
    fn density(&self) -> f64 {
        self.population() as f64 / self.width() as f64
    }

    /// Spatial entropy based on k-block frequencies
    /// Measures how "random" the spatial pattern is
    /// Returns bits per block; max is k for uniform distribution
    fn block_entropy(&self, k: usize) -> f64 {
        if k == 0 || k > self.width() {
            return 0.0;
        }

        // Count occurrences of each k-bit pattern (with wraparound)
        let mut counts = vec![0usize; 1 << k];
        let n = self.width();

        for i in 0..n {
            let mut pattern = 0usize;
            for j in 0..k {
                if self.cells[(i + j) % n] {
                    pattern |= 1 << (k - 1 - j);
                }
            }
            counts[pattern] += 1;
        }

        // Compute Shannon entropy: H = -Σ p_i log2(p_i)
        let total = n as f64;
        let mut entropy = 0.0;
        for &count in &counts {
            if count > 0 {
                let p = count as f64 / total;
                entropy -= p * p.log2();
            }
        }

        entropy
    }

    /// Convert state to a compact hash for cycle detection
    fn state_hash(&self) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        let mut hasher = DefaultHasher::new();
        self.cells.hash(&mut hasher);
        hasher.finish()
    }
}

impl Hash for Automaton {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.cells.hash(state);
    }
}

/// Result of running a CA until it cycles or reaches max steps
#[derive(Debug)]
struct CycleAnalysis {
    /// Steps before entering cycle (transient length)
    transient: usize,
    /// Length of the cycle (0 if didn't find one)
    period: usize,
    /// Whether the CA died (all zeros)
    died: bool,
    /// Final density
    final_density: f64,
}

/// Run CA until it enters a cycle or hits max_steps
fn find_cycle(rule: u8, width: usize, max_steps: usize) -> CycleAnalysis {
    let mut ca = Automaton::new(width, rule);
    let mut seen: HashSet<Vec<bool>> = HashSet::new();
    let mut history: Vec<Vec<bool>> = Vec::new();

    seen.insert(ca.cells.clone());
    history.push(ca.cells.clone());

    for step in 0..max_steps {
        ca.step();

        // Check if died
        if ca.population() == 0 {
            return CycleAnalysis {
                transient: step + 1,
                period: 1, // stays dead
                died: true,
                final_density: 0.0,
            };
        }

        // Check if we've seen this state before
        if seen.contains(&ca.cells) {
            // Find where in history this state first appeared
            let cycle_start = history.iter().position(|s| s == &ca.cells).unwrap();
            return CycleAnalysis {
                transient: cycle_start,
                period: step + 1 - cycle_start,
                died: false,
                final_density: ca.density(),
            };
        }

        seen.insert(ca.cells.clone());
        history.push(ca.cells.clone());
    }

    // Didn't find cycle within max_steps
    CycleAnalysis {
        transient: max_steps,
        period: 0,
        died: false,
        final_density: ca.density(),
    }
}

impl fmt::Display for Automaton {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for &cell in &self.cells {
            write!(f, "{}", if cell { '#' } else { ' ' })?;
        }
        Ok(())
    }
}

/// Run a CA for n generations and print each row
fn run_and_display(rule: u8, width: usize, generations: usize) {
    println!("Rule {rule}");
    println!("{}", "-".repeat(width));

    let mut ca = Automaton::new(width, rule);
    println!("{ca}");

    for _ in 0..generations {
        ca.step();
        println!("{ca}");
    }

    println!("{}", "-".repeat(width));
}

/// The "interesting" rules - Wolfram's Class 3 and 4
const INTERESTING_RULES: [u8; 12] = [
    30,  // Class 3: chaotic
    45,  // Class 3: chaotic
    60,  // Class 3: chaotic (XOR)
    73,  // Class 4: complex
    89,  // Class 4: complex
    90,  // Class 3: Sierpinski triangle
    105, // Class 3: chaotic
    106, // Class 4: complex
    110, // Class 4: Turing complete!
    124, // Class 4: complex
    137, // Class 4: complex
    150, // Class 3: chaotic
];

fn main() {
    let args: Vec<String> = std::env::args().collect();

    // Check for special modes
    if args.get(1).map(|s| s.as_str()) == Some("--analyze") {
        // Analyze all 256 rules for cycle behavior
        let width: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(31);
        let max_steps: usize = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(1000);

        println!("Analyzing all 256 rules (width={width}, max_steps={max_steps})");
        println!("{:>4} {:>10} {:>8} {:>6} {:>8}", "Rule", "Transient", "Period", "Died?", "Density");
        println!("{}", "-".repeat(50));

        let mut class_counts = [0usize; 4]; // die, short cycle, long cycle, no cycle found

        for rule in 0..=255u8 {
            let analysis = find_cycle(rule, width, max_steps);

            let class = if analysis.died {
                0
            } else if analysis.period > 0 && analysis.period <= 10 {
                1
            } else if analysis.period > 10 {
                2
            } else {
                3
            };
            class_counts[class] += 1;

            // Only print interesting rules (not immediately dying, or complex behavior)
            if !analysis.died || analysis.transient > 1 {
                println!(
                    "{:>4} {:>10} {:>8} {:>6} {:>8.3}",
                    rule,
                    analysis.transient,
                    if analysis.period > 0 {
                        analysis.period.to_string()
                    } else {
                        ">max".to_string()
                    },
                    if analysis.died { "yes" } else { "no" },
                    analysis.final_density
                );
            }
        }

        println!("{}", "-".repeat(50));
        println!("Summary:");
        println!("  Dies immediately: {}", class_counts[0]);
        println!("  Short cycle (<=10): {}", class_counts[1]);
        println!("  Long cycle (>10): {}", class_counts[2]);
        println!("  No cycle found: {}", class_counts[3]);

        return;
    }

    if args.get(1).map(|s| s.as_str()) == Some("--cycle") {
        // Analyze single rule for cycle
        let rule: u8 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(110);
        let width: usize = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(31);
        let max_steps: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(10000);

        println!("Analyzing Rule {rule} (width={width}, max_steps={max_steps})");
        let analysis = find_cycle(rule, width, max_steps);

        println!("  Transient length: {}", analysis.transient);
        if analysis.period > 0 {
            println!("  Cycle period: {}", analysis.period);
        } else {
            println!("  Cycle period: not found within {max_steps} steps");
        }
        println!("  Died: {}", if analysis.died { "yes" } else { "no" });
        println!("  Final density: {:.3}", analysis.final_density);

        return;
    }

    if args.get(1).map(|s| s.as_str()) == Some("--entropy") {
        // Track entropy over time for a rule
        let rule: u8 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(110);
        let width: usize = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(79);
        let generations: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(100);
        let block_size: usize = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(3);

        println!("Entropy analysis: Rule {rule} (width={width}, blocks={block_size})");
        println!("Max possible entropy: {:.3} bits", block_size as f64);
        println!("{:>5} {:>8} {:>8}", "Gen", "Entropy", "Density");
        println!("{}", "-".repeat(25));

        let mut ca = Automaton::new(width, rule);
        let mut entropies = Vec::with_capacity(generations + 1);

        let h = ca.block_entropy(block_size);
        entropies.push(h);
        println!("{:>5} {:>8.4} {:>8.3}", 0, h, ca.density());

        for g in 1..=generations {
            ca.step();
            let h = ca.block_entropy(block_size);
            entropies.push(h);

            // Print every 10th generation, plus first few and last
            if g <= 5 || g % 10 == 0 || g == generations {
                println!("{:>5} {:>8.4} {:>8.3}", g, h, ca.density());
            }
        }

        // Summary statistics
        println!("{}", "-".repeat(25));
        let mean: f64 = entropies.iter().sum::<f64>() / entropies.len() as f64;
        let variance: f64 = entropies.iter().map(|h| (h - mean).powi(2)).sum::<f64>()
            / entropies.len() as f64;
        let min = entropies.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = entropies.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

        println!("Mean entropy:  {:.4}", mean);
        println!("Std dev:       {:.4}", variance.sqrt());
        println!("Range:         [{:.4}, {:.4}]", min, max);
        println!("Normalized:    {:.1}% of max", 100.0 * mean / block_size as f64);

        return;
    }

    if args.get(1).map(|s| s.as_str()) == Some("--entropy-survey") {
        // Survey all 256 rules by entropy signature
        let width: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(79);
        let generations: usize = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(100);
        let block_size: usize = 3;
        let max_entropy = block_size as f64;

        println!("Entropy survey (width={width}, gens={generations}, blocks={block_size})");
        println!("{:>4} {:>7} {:>7} {:>8}", "Rule", "Mean", "StdDev", "Class");
        println!("{}", "-".repeat(32));

        let mut classes: [Vec<u8>; 5] = Default::default(); // dead, periodic, fractal, complex, chaotic

        for rule in 0..=255u8 {
            let mut ca = Automaton::new(width, rule);
            let mut entropies = Vec::with_capacity(generations + 1);

            // Skip transient phase (first 50 generations)
            let skip = 50;
            for _ in 0..skip {
                ca.step();
            }

            entropies.push(ca.block_entropy(block_size));
            for _ in 0..generations {
                ca.step();
                entropies.push(ca.block_entropy(block_size));
            }

            let mean: f64 = entropies.iter().sum::<f64>() / entropies.len() as f64;
            let variance: f64 = entropies.iter().map(|h| (h - mean).powi(2)).sum::<f64>()
                / entropies.len() as f64;
            let std_dev = variance.sqrt();
            let norm_mean = mean / max_entropy;
            let norm_std = std_dev / max_entropy;

            // Classify based on entropy signature
            let (class_idx, class_name) = if norm_mean < 0.05 {
                (0, "dead")
            } else if norm_std < 0.02 && norm_mean < 0.3 {
                (1, "periodic")
            } else if norm_std > 0.15 {
                (2, "fractal")
            } else if norm_mean > 0.75 && norm_std < 0.1 {
                (4, "chaotic")
            } else {
                (3, "complex")
            };

            classes[class_idx].push(rule);

            // Only print interesting rules
            if class_idx >= 2 {
                println!(
                    "{:>4} {:>7.3} {:>7.3} {:>8}",
                    rule, norm_mean, norm_std, class_name
                );
            }
        }

        println!("{}", "-".repeat(32));
        println!("Classification:");
        println!("  Dead:     {} rules", classes[0].len());
        println!("  Periodic: {} rules", classes[1].len());
        println!("  Fractal:  {} rules ({:?}...)", classes[2].len(),
            &classes[2][..classes[2].len().min(5)]);
        println!("  Complex:  {} rules", classes[3].len());
        println!("  Chaotic:  {} rules ({:?})", classes[4].len(), classes[4]);

        return;
    }

    // Default: visualize a single rule
    let rule: u8 = args
        .get(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(110);

    let width: usize = args
        .get(2)
        .and_then(|s| s.parse().ok())
        .unwrap_or(79);

    let generations: usize = args
        .get(3)
        .and_then(|s| s.parse().ok())
        .unwrap_or(40);

    run_and_display(rule, width, generations);

    // Show what makes this rule tick
    println!("\nRule {rule} transition table:");
    println!("  neighborhood -> next");
    for i in (0..8).rev() {
        let pattern = format!(
            "{}{}{}",
            (i >> 2) & 1,
            (i >> 1) & 1,
            i & 1
        );
        let result = (rule >> i) & 1;
        println!("      {pattern}      ->  {result}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rule_110_known_sequence() {
        // Rule 110 from single cell should produce known pattern
        let mut ca = Automaton::new(7, 110);
        // Initial: ...#...
        assert_eq!(format!("{ca}"), "   #   ");

        ca.step();
        // After 1 step: ..##...
        assert_eq!(format!("{ca}"), "  ##   ");

        ca.step();
        // After 2 steps: .###...
        assert_eq!(format!("{ca}"), " ###   ");

        ca.step();
        // After 3 steps: ##.#...
        assert_eq!(format!("{ca}"), "## #   ");
    }

    #[test]
    fn test_rule_90_sierpinski() {
        // Rule 90 produces XOR / Sierpinski pattern
        let mut ca = Automaton::new(7, 90);
        ca.step();
        // Should have two cells on either side of center
        assert_eq!(format!("{ca}"), "  # #  ");
    }

    #[test]
    fn test_wrap_around() {
        // Test that edges wrap
        let ca = Automaton::from_cells(vec![true, false, false, false, false], 110);
        // Cell at index 0: neighborhood is (cell[4], cell[0], cell[1]) = (0, 1, 0)
        // index = 0*4 + 1*2 + 0*1 = 2
        // Rule 110 = 0b01101110, bit 2 = 1
        // So cell 0 should become 1
        let mut ca = ca;
        ca.step();
        assert!(ca.cells[0]);
    }

    #[test]
    fn test_all_rules_deterministic() {
        // Every rule should be deterministic
        for rule in 0..=255u8 {
            let mut ca1 = Automaton::new(20, rule);
            let mut ca2 = Automaton::new(20, rule);
            for _ in 0..10 {
                ca1.step();
                ca2.step();
            }
            assert_eq!(ca1.cells, ca2.cells);
        }
    }
}
