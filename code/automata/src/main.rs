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

use flate2::write::DeflateEncoder;
use flate2::Compression;
use std::collections::HashSet;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::io::Write;

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

/// Compression analysis: how well does the spacetime diagram compress?
/// Returns (raw_bits, compressed_bits, ratio)
fn compression_ratio(rule: u8, width: usize, generations: usize) -> (usize, usize, f64) {
    let mut ca = Automaton::new(width, rule);

    // Pack spacetime into bytes (8 cells per byte)
    let total_cells = width * (generations + 1);
    let mut raw_bytes = Vec::with_capacity((total_cells + 7) / 8);

    let mut current_byte = 0u8;
    let mut bit_pos = 0;

    // Helper to flush bits to bytes
    let flush_cell = |cell: bool, byte: &mut u8, pos: &mut usize, bytes: &mut Vec<u8>| {
        if cell {
            *byte |= 1 << (7 - *pos);
        }
        *pos += 1;
        if *pos == 8 {
            bytes.push(*byte);
            *byte = 0;
            *pos = 0;
        }
    };

    // First generation
    for &cell in &ca.cells {
        flush_cell(cell, &mut current_byte, &mut bit_pos, &mut raw_bytes);
    }

    // Subsequent generations
    for _ in 0..generations {
        ca.step();
        for &cell in &ca.cells {
            flush_cell(cell, &mut current_byte, &mut bit_pos, &mut raw_bytes);
        }
    }

    // Flush remaining bits
    if bit_pos > 0 {
        raw_bytes.push(current_byte);
    }

    // Compress with deflate
    let mut encoder = DeflateEncoder::new(Vec::new(), Compression::best());
    encoder.write_all(&raw_bytes).unwrap();
    let compressed = encoder.finish().unwrap();

    let raw_bits = total_cells;
    let compressed_bits = compressed.len() * 8;
    let ratio = compressed_bits as f64 / raw_bits as f64;

    (raw_bits, compressed_bits, ratio)
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

    if args.get(1).map(|s| s.as_str()) == Some("--compress") {
        // Compression analysis for single rule
        let rule: u8 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(110);
        let width: usize = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(79);
        let generations: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(200);

        println!("Compression analysis: Rule {rule} (width={width}, gens={generations})");
        let (raw, compressed, ratio) = compression_ratio(rule, width, generations);

        println!("  Raw size:        {} bits", raw);
        println!("  Compressed:      {} bits", compressed);
        println!("  Ratio:           {:.3} (lower = more compressible)", ratio);
        println!("  Incompressible:  {:.1}%", ratio * 100.0);

        return;
    }

    if args.get(1).map(|s| s.as_str()) == Some("--infer") {
        // Infer rule from observations: can we recover the causal mechanism?
        // This tests whether we can learn the rule vs. just correlations
        let rule: u8 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(110);
        let width: usize = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(50);
        let generations: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(20);
        let noise: f64 = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(0.0);

        println!("Rule inference test (true rule={rule}, width={width}, gens={generations}, noise={noise})");

        // Generate training data from random initial conditions
        let mut observations: [usize; 8] = [0; 8]; // count of 0->? and 1->? for each neighborhood
        let mut outcomes: [usize; 8] = [0; 8];     // count of 1 outcomes for each neighborhood

        // Run multiple random initial conditions
        let num_trials = 10;
        for trial in 0..num_trials {
            // Random initial state
            let seed: usize = trial * 12345 + 67890;
            let cells: Vec<bool> = (0..width)
                .map(|i| ((seed.wrapping_mul(i + 1)) % 100) < 50)
                .collect();

            let mut ca = Automaton::from_cells(cells, rule);

            for _ in 0..generations {
                // Observe all neighborhoods and their outcomes
                let n = ca.width();
                let old_cells = ca.cells.clone();
                ca.step();

                for i in 0..n {
                    let left = old_cells[(i + n - 1) % n];
                    let center = old_cells[i];
                    let right = old_cells[(i + 1) % n];
                    let neighborhood = (left as usize) << 2 | (center as usize) << 1 | (right as usize);

                    observations[neighborhood] += 1;

                    // Apply noise: with probability `noise`, flip the observed outcome
                    let mut outcome = ca.cells[i];
                    if noise > 0.0 {
                        let noise_check = ((seed + i + observations[neighborhood]) % 1000) as f64 / 1000.0;
                        if noise_check < noise {
                            outcome = !outcome;
                        }
                    }
                    if outcome {
                        outcomes[neighborhood] += 1;
                    }
                }
            }
        }

        // Infer rule: majority vote for each neighborhood
        let mut inferred_rule: u8 = 0;
        println!("\nNeighborhood observations:");
        println!("  NHD   Count   P(1)   Inferred   True");
        println!("{}", "-".repeat(45));

        for i in 0..8 {
            let count = observations[i];
            let ones = outcomes[i];
            let p = if count > 0 { ones as f64 / count as f64 } else { 0.5 };
            let inferred_bit = if p > 0.5 { 1 } else { 0 };
            let true_bit = (rule >> i) & 1;

            if inferred_bit == 1 {
                inferred_rule |= 1 << i;
            }

            let pattern = format!("{}{}{}", (i >> 2) & 1, (i >> 1) & 1, i & 1);
            let match_mark = if inferred_bit == true_bit { "✓" } else { "✗" };
            println!(
                "  {}   {:>6}   {:.3}      {}          {} {}",
                pattern, count, p, inferred_bit, true_bit, match_mark
            );
        }

        println!("{}", "-".repeat(45));
        println!("Inferred rule: {}", inferred_rule);
        println!("True rule:     {}", rule);
        println!("Match:         {}", if inferred_rule == rule { "EXACT" } else { "MISMATCH" });

        // Now test generalization: does the inferred rule work on a different distribution?
        println!("\nGeneralization test (biased initial conditions):");

        // Test on sparse initial conditions (10% density instead of 50%)
        let sparse_density = 10;
        let mut errors = 0;
        let mut total = 0;

        for trial in 0..5 {
            let seed: usize = trial * 99999 + 11111;
            let cells: Vec<bool> = (0..width)
                .map(|i| ((seed.wrapping_mul(i + 1)) % 100) < sparse_density)
                .collect();

            let mut ca_true = Automaton::from_cells(cells.clone(), rule);
            let mut ca_inferred = Automaton::from_cells(cells, inferred_rule);

            for _ in 0..generations {
                ca_true.step();
                ca_inferred.step();

                for i in 0..width {
                    total += 1;
                    if ca_true.cells[i] != ca_inferred.cells[i] {
                        errors += 1;
                    }
                }
            }
        }

        let error_rate = errors as f64 / total as f64;
        println!("  Sparse ({}% density): {:.4}% error rate", sparse_density, error_rate * 100.0);

        // Test on dense initial conditions (90% density)
        let dense_density = 90;
        errors = 0;
        total = 0;

        for trial in 0..5 {
            let seed: usize = trial * 77777 + 33333;
            let cells: Vec<bool> = (0..width)
                .map(|i| ((seed.wrapping_mul(i + 1)) % 100) < dense_density)
                .collect();

            let mut ca_true = Automaton::from_cells(cells.clone(), rule);
            let mut ca_inferred = Automaton::from_cells(cells, inferred_rule);

            for _ in 0..generations {
                ca_true.step();
                ca_inferred.step();

                for i in 0..width {
                    total += 1;
                    if ca_true.cells[i] != ca_inferred.cells[i] {
                        errors += 1;
                    }
                }
            }
        }

        let error_rate = errors as f64 / total as f64;
        println!("  Dense ({}% density):  {:.4}% error rate", dense_density, error_rate * 100.0);

        // Compare with a "correlational" baseline that uses global features
        println!("\nCorrelational baseline (global features only):");

        // Train a simple correlational model: P(cell=1 | current_cell, density_bucket)
        // This captures global statistics but not the local mechanism
        let mut corr_counts: [[usize; 2]; 10] = [[0; 2]; 10]; // [density_bucket][current_cell] -> count
        let mut corr_ones: [[usize; 2]; 10] = [[0; 2]; 10];   // count of 1 outcomes

        for trial in 0..num_trials {
            let seed: usize = trial * 12345 + 67890;
            let cells: Vec<bool> = (0..width)
                .map(|i| ((seed.wrapping_mul(i + 1)) % 100) < 50)
                .collect();

            let mut ca = Automaton::from_cells(cells, rule);

            for _ in 0..generations {
                let old_cells = ca.cells.clone();
                let density = old_cells.iter().filter(|&&c| c).count() as f64 / width as f64;
                let bucket = ((density * 10.0) as usize).min(9);

                ca.step();

                for i in 0..width {
                    let curr = old_cells[i] as usize;
                    corr_counts[bucket][curr] += 1;
                    if ca.cells[i] {
                        corr_ones[bucket][curr] += 1;
                    }
                }
            }
        }

        // Test correlational predictor on OOD data
        let mut corr_errors_sparse = 0;
        let mut corr_errors_dense = 0;
        let mut corr_total_sparse = 0;
        let mut corr_total_dense = 0;

        for trial in 0..5 {
            // Sparse test
            let seed: usize = trial * 99999 + 11111;
            let cells: Vec<bool> = (0..width)
                .map(|i| ((seed.wrapping_mul(i + 1)) % 100) < sparse_density)
                .collect();

            let mut ca = Automaton::from_cells(cells, rule);

            for _ in 0..generations {
                let old_cells = ca.cells.clone();
                let density = old_cells.iter().filter(|&&c| c).count() as f64 / width as f64;
                let bucket = ((density * 10.0) as usize).min(9);

                ca.step();

                for i in 0..width {
                    let curr = old_cells[i] as usize;
                    // Predict using correlational model
                    let count = corr_counts[bucket][curr];
                    let ones = corr_ones[bucket][curr];
                    let predicted = if count > 0 { ones > count / 2 } else { false };

                    corr_total_sparse += 1;
                    if predicted != ca.cells[i] {
                        corr_errors_sparse += 1;
                    }
                }
            }

            // Dense test
            let seed: usize = trial * 77777 + 33333;
            let cells: Vec<bool> = (0..width)
                .map(|i| ((seed.wrapping_mul(i + 1)) % 100) < dense_density)
                .collect();

            let mut ca = Automaton::from_cells(cells, rule);

            for _ in 0..generations {
                let old_cells = ca.cells.clone();
                let density = old_cells.iter().filter(|&&c| c).count() as f64 / width as f64;
                let bucket = ((density * 10.0) as usize).min(9);

                ca.step();

                for i in 0..width {
                    let curr = old_cells[i] as usize;
                    let count = corr_counts[bucket][curr];
                    let ones = corr_ones[bucket][curr];
                    let predicted = if count > 0 { ones > count / 2 } else { false };

                    corr_total_dense += 1;
                    if predicted != ca.cells[i] {
                        corr_errors_dense += 1;
                    }
                }
            }
        }

        let corr_rate_sparse = corr_errors_sparse as f64 / corr_total_sparse as f64;
        let corr_rate_dense = corr_errors_dense as f64 / corr_total_dense as f64;
        println!("  Sparse ({}% density): {:.2}% error rate", sparse_density, corr_rate_sparse * 100.0);
        println!("  Dense ({}% density):  {:.2}% error rate", dense_density, corr_rate_dense * 100.0);

        // Recalculate causal errors for fair comparison
        let mut causal_errors_sparse = 0;
        let mut causal_errors_dense = 0;

        for trial in 0..5 {
            let seed: usize = trial * 99999 + 11111;
            let cells: Vec<bool> = (0..width)
                .map(|i| ((seed.wrapping_mul(i + 1)) % 100) < sparse_density)
                .collect();

            let mut ca_true = Automaton::from_cells(cells.clone(), rule);
            let mut ca_inferred = Automaton::from_cells(cells, inferred_rule);

            for _ in 0..generations {
                ca_true.step();
                ca_inferred.step();
                for i in 0..width {
                    if ca_true.cells[i] != ca_inferred.cells[i] {
                        causal_errors_sparse += 1;
                    }
                }
            }

            let seed: usize = trial * 77777 + 33333;
            let cells: Vec<bool> = (0..width)
                .map(|i| ((seed.wrapping_mul(i + 1)) % 100) < dense_density)
                .collect();

            let mut ca_true = Automaton::from_cells(cells.clone(), rule);
            let mut ca_inferred = Automaton::from_cells(cells, inferred_rule);

            for _ in 0..generations {
                ca_true.step();
                ca_inferred.step();
                for i in 0..width {
                    if ca_true.cells[i] != ca_inferred.cells[i] {
                        causal_errors_dense += 1;
                    }
                }
            }
        }

        let causal_rate_sparse = causal_errors_sparse as f64 / corr_total_sparse as f64;
        let causal_rate_dense = causal_errors_dense as f64 / corr_total_dense as f64;

        println!("\nComparison (OOD generalization):");
        println!("  {:20} {:>10} {:>10}", "Learner", "Sparse", "Dense");
        println!("  {:20} {:>9.2}% {:>9.2}%", "Local (causal)", causal_rate_sparse * 100.0, causal_rate_dense * 100.0);
        println!("  {:20} {:>9.2}% {:>9.2}%", "Global (correlational)", corr_rate_sparse * 100.0, corr_rate_dense * 100.0);

        if inferred_rule == rule {
            println!("\n→ Rule recovery successful: learned causal mechanism, not just correlations.");
        } else {
            println!("\n→ Rule recovery failed: noise or insufficient data prevented causal learning.");
        }

        return;
    }

    if args.get(1).map(|s| s.as_str()) == Some("--radius") {
        // Infer the radius (locality) of a rule from observations alone
        // Key question: can we discover that ECAs use 3-cell neighborhoods?
        let rule: u8 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(110);
        let width: usize = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(50);
        let generations: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(20);
        let max_radius: usize = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(4);

        println!("Radius inference (true rule={rule}, width={width}, gens={generations})");
        println!("Testing radii 0 to {max_radius}...\n");

        // Generate observations from random initial conditions
        let num_trials = 10;
        let mut transitions: Vec<(Vec<bool>, Vec<bool>)> = Vec::new();

        for trial in 0..num_trials {
            let seed: usize = trial * 12345 + 67890;
            let cells: Vec<bool> = (0..width)
                .map(|i| ((seed.wrapping_mul(i + 1)) % 100) < 50)
                .collect();

            let mut ca = Automaton::from_cells(cells, rule);

            for _ in 0..generations {
                let before = ca.cells.clone();
                ca.step();
                let after = ca.cells.clone();
                transitions.push((before, after));
            }
        }

        println!("Collected {} row transitions\n", transitions.len());

        // For each candidate radius, check if the mapping is consistent
        use std::collections::HashMap;

        for r in 0..=max_radius {
            let window_size = 2 * r + 1;
            let mut mapping: HashMap<Vec<bool>, (usize, usize)> = HashMap::new(); // window -> (count_0, count_1)

            for (before, after) in &transitions {
                let n = before.len();
                for i in 0..n {
                    // Extract window of radius r around cell i (with wraparound)
                    let window: Vec<bool> = (0..window_size)
                        .map(|j| {
                            let idx = (i + n - r + j) % n;
                            before[idx]
                        })
                        .collect();

                    let output = after[i];
                    let entry = mapping.entry(window).or_insert((0, 0));
                    if output {
                        entry.1 += 1;
                    } else {
                        entry.0 += 1;
                    }
                }
            }

            // Check consistency: each window should map to only one output
            let total_windows = mapping.len();
            let inconsistent: Vec<_> = mapping
                .iter()
                .filter(|(_, (zeros, ones))| *zeros > 0 && *ones > 0)
                .collect();

            let consistent = inconsistent.is_empty();
            let consistency_rate = (total_windows - inconsistent.len()) as f64 / total_windows as f64;

            println!("Radius {r} (window size {window_size}):");
            println!("  Unique windows observed: {total_windows} / {} possible", 1usize << window_size);
            println!("  Consistent: {} ({:.1}%)",
                if consistent { "YES" } else { "NO" },
                consistency_rate * 100.0
            );

            if !consistent {
                println!("  Inconsistent windows: {} (examples below)", inconsistent.len());
                for (window, (zeros, ones)) in inconsistent.iter().take(3) {
                    let pattern: String = window.iter().map(|&b| if b { '1' } else { '0' }).collect();
                    println!("    {} -> 0 ({} times), 1 ({} times)", pattern, zeros, ones);
                }
            }

            println!();

            // If consistent, we found the minimal radius
            if consistent {
                println!("→ Inferred radius: {r}");
                println!("  (True ECA radius is 1)");
                if r == 1 {
                    println!("  SUCCESS: Correctly identified 3-cell neighborhood");
                } else if r < 1 {
                    println!("  INTERESTING: Rule has effective radius < 1 (some neighbors don't matter)");
                } else {
                    println!("  NOTE: Found consistent at r={r}, but r=1 should suffice for ECAs");
                }
                break;
            }
        }

        return;
    }

    if args.get(1).map(|s| s.as_str()) == Some("--dependency") {
        // Analyze which positions in the neighborhood actually matter
        // For each rule, determine: does output depend on left? center? right?

        println!("Dependency analysis for all 256 rules");
        println!("Checking which neighborhood positions are necessary...\n");

        #[derive(Debug, Clone, Copy, PartialEq)]
        struct Dependencies {
            left: bool,
            center: bool,
            right: bool,
        }

        let mut dep_counts: std::collections::HashMap<(bool, bool, bool), Vec<u8>> =
            std::collections::HashMap::new();

        for rule in 0..=255u8 {
            // For each position, check if changing it ever changes the output
            // while holding other positions fixed

            // Left matters if there exist (c,r) such that rule(0,c,r) ≠ rule(1,c,r)
            let left_matters = (0..4).any(|cr| {
                let c = (cr >> 1) & 1;
                let r = cr & 1;
                let n0 = (0 << 2) | (c << 1) | r; // left=0
                let n1 = (1 << 2) | (c << 1) | r; // left=1
                ((rule >> n0) & 1) != ((rule >> n1) & 1)
            });

            // Center matters if there exist (l,r) such that rule(l,0,r) ≠ rule(l,1,r)
            let center_matters = (0..4).any(|lr| {
                let l = (lr >> 1) & 1;
                let r = lr & 1;
                let n0 = (l << 2) | (0 << 1) | r; // center=0
                let n1 = (l << 2) | (1 << 1) | r; // center=1
                ((rule >> n0) & 1) != ((rule >> n1) & 1)
            });

            // Right matters if there exist (l,c) such that rule(l,c,0) ≠ rule(l,c,1)
            let right_matters = (0..4).any(|lc| {
                let l = (lc >> 1) & 1;
                let c = lc & 1;
                let n0 = (l << 2) | (c << 1) | 0; // right=0
                let n1 = (l << 2) | (c << 1) | 1; // right=1
                ((rule >> n0) & 1) != ((rule >> n1) & 1)
            });

            dep_counts
                .entry((left_matters, center_matters, right_matters))
                .or_default()
                .push(rule);
        }

        // Print results by dependency pattern
        let patterns = [
            ((false, false, false), "none (constant)"),
            ((false, true, false), "center only"),
            ((true, false, false), "left only"),
            ((false, false, true), "right only"),
            ((true, true, false), "left + center"),
            ((false, true, true), "center + right"),
            ((true, false, true), "left + right (symmetric)"),
            ((true, true, true), "all three"),
        ];

        for ((l, c, r), name) in patterns {
            if let Some(rules) = dep_counts.get(&(l, c, r)) {
                println!("{}: {} rules", name, rules.len());
                if rules.len() <= 16 {
                    for chunk in rules.chunks(8) {
                        let s: String = chunk.iter().map(|r| format!("{:>4}", r)).collect::<Vec<_>>().join("");
                        println!("    {}", s);
                    }
                } else {
                    println!("    (first 8: {:?}...)", &rules[..8]);
                }
                println!();
            }
        }

        // Interesting follow-up: for "left + right" rules (ignoring center),
        // what Boolean functions of (left, right) do they implement?
        println!("Analysis of center-ignoring rules (left + right only):");
        if let Some(rules) = dep_counts.get(&(true, false, true)) {
            for &rule in rules {
                // The rule is a function of (left, right) only
                // For each (l, r), what's the output?
                let mut f = String::new();
                for lr in 0..4 {
                    let l = (lr >> 1) & 1;
                    let r = lr & 1;
                    // Output should be same for both center values
                    let n0 = (l << 2) | (0 << 1) | r;
                    let out = (rule >> n0) & 1;
                    f.push(if out == 1 { '1' } else { '0' });
                }
                let func_name = match f.as_str() {
                    "0000" => "FALSE",
                    "1111" => "TRUE",
                    "0001" => "NOR",
                    "0010" => "l AND NOT r",
                    "0011" => "NOT r",
                    "0100" => "NOT l AND r",
                    "0101" => "NOT l",
                    "0110" => "XOR",
                    "0111" => "NAND",
                    "1000" => "AND",
                    "1001" => "XNOR",
                    "1010" => "l",
                    "1011" => "l OR NOT r",
                    "1100" => "r",
                    "1101" => "NOT l OR r",
                    "1110" => "OR",
                    _ => "?",
                };
                println!("  Rule {:>3}: f(l,r) = {} ({})", rule, f, func_name);
            }
        }

        return;
    }

    if args.get(1).map(|s| s.as_str()) == Some("--dependency-infer") {
        // Infer which neighborhood positions matter from observations alone
        // (Statistical inference vs. direct rule analysis)
        let rule: u8 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(90);
        let width: usize = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(50);
        let generations: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(30);

        println!("Dependency inference from observations (rule={rule})");
        println!("(Not examining rule directly—only observing behavior)\n");

        // Generate observations
        let num_trials = 10;
        let mut transitions: Vec<(Vec<bool>, Vec<bool>)> = Vec::new();

        for trial in 0..num_trials {
            let seed: usize = trial * 12345 + 67890;
            let cells: Vec<bool> = (0..width)
                .map(|i| ((seed.wrapping_mul(i + 1)) % 100) < 50)
                .collect();

            let mut ca = Automaton::from_cells(cells, rule);

            for _ in 0..generations {
                let before = ca.cells.clone();
                ca.step();
                let after = ca.cells.clone();
                transitions.push((before, after));
            }
        }

        println!("Collected {} transitions\n", transitions.len());

        // For each position (left, center, right), check if it affects the output
        // Method: group observations by the other two positions, see if this position's
        // value correlates with different outputs

        // Count (l, c, r) -> output patterns
        use std::collections::HashMap;
        let mut counts: HashMap<(bool, bool, bool), (usize, usize)> = HashMap::new();

        for (before, after) in &transitions {
            let n = before.len();
            for i in 0..n {
                let left = before[(i + n - 1) % n];
                let center = before[i];
                let right = before[(i + 1) % n];
                let output = after[i];

                let entry = counts.entry((left, center, right)).or_insert((0, 0));
                if output {
                    entry.1 += 1;
                } else {
                    entry.0 += 1;
                }
            }
        }

        // Check if left matters: for each (c, r), compare outputs when left differs
        println!("Testing whether LEFT matters:");
        let mut left_matters = false;
        for c in [false, true] {
            for r in [false, true] {
                let out_0 = counts.get(&(false, c, r)).map(|(z, o)| *o > *z);
                let out_1 = counts.get(&(true, c, r)).map(|(z, o)| *o > *z);
                if out_0 != out_1 {
                    println!("  At (c={}, r={}): left=0 → {:?}, left=1 → {:?} — DIFFERENT",
                        c as u8, r as u8, out_0, out_1);
                    left_matters = true;
                }
            }
        }
        if !left_matters {
            println!("  No differences found — LEFT does NOT matter");
        }
        println!();

        // Check if center matters
        println!("Testing whether CENTER matters:");
        let mut center_matters = false;
        for l in [false, true] {
            for r in [false, true] {
                let out_0 = counts.get(&(l, false, r)).map(|(z, o)| *o > *z);
                let out_1 = counts.get(&(l, true, r)).map(|(z, o)| *o > *z);
                if out_0 != out_1 {
                    println!("  At (l={}, r={}): center=0 → {:?}, center=1 → {:?} — DIFFERENT",
                        l as u8, r as u8, out_0, out_1);
                    center_matters = true;
                }
            }
        }
        if !center_matters {
            println!("  No differences found — CENTER does NOT matter");
        }
        println!();

        // Check if right matters
        println!("Testing whether RIGHT matters:");
        let mut right_matters = false;
        for l in [false, true] {
            for c in [false, true] {
                let out_0 = counts.get(&(l, c, false)).map(|(z, o)| *o > *z);
                let out_1 = counts.get(&(l, c, true)).map(|(z, o)| *o > *z);
                if out_0 != out_1 {
                    println!("  At (l={}, c={}): right=0 → {:?}, right=1 → {:?} — DIFFERENT",
                        l as u8, c as u8, out_0, out_1);
                    right_matters = true;
                }
            }
        }
        if !right_matters {
            println!("  No differences found — RIGHT does NOT matter");
        }
        println!();

        // Summary
        let deps: Vec<&str> = [
            if left_matters { Some("left") } else { None },
            if center_matters { Some("center") } else { None },
            if right_matters { Some("right") } else { None },
        ].into_iter().flatten().collect();

        if deps.is_empty() {
            println!("→ Inferred: CONSTANT rule (no dependencies)");
        } else {
            println!("→ Inferred dependencies: {}", deps.join(" + "));
        }

        // Compare with ground truth
        println!("\nGround truth (from rule {rule} = 0b{:08b}):", rule);
        let true_left = (0..4).any(|cr| {
            let c = (cr >> 1) & 1;
            let r = cr & 1;
            let n0 = (0 << 2) | (c << 1) | r;
            let n1 = (1 << 2) | (c << 1) | r;
            ((rule >> n0) & 1) != ((rule >> n1) & 1)
        });
        let true_center = (0..4).any(|lr| {
            let l = (lr >> 1) & 1;
            let r = lr & 1;
            let n0 = (l << 2) | (0 << 1) | r;
            let n1 = (l << 2) | (1 << 1) | r;
            ((rule >> n0) & 1) != ((rule >> n1) & 1)
        });
        let true_right = (0..4).any(|lc| {
            let l = (lc >> 1) & 1;
            let c = lc & 1;
            let n0 = (l << 2) | (c << 1) | 0;
            let n1 = (l << 2) | (c << 1) | 1;
            ((rule >> n0) & 1) != ((rule >> n1) & 1)
        });

        let true_deps: Vec<&str> = [
            if true_left { Some("left") } else { None },
            if true_center { Some("center") } else { None },
            if true_right { Some("right") } else { None },
        ].into_iter().flatten().collect();

        if true_deps.is_empty() {
            println!("  True dependencies: CONSTANT");
        } else {
            println!("  True dependencies: {}", true_deps.join(" + "));
        }

        let match_result = left_matters == true_left && center_matters == true_center && right_matters == true_right;
        println!("  Match: {}", if match_result { "YES" } else { "NO" });

        return;
    }

    if args.get(1).map(|s| s.as_str()) == Some("--radius-survey") {
        // Survey all 256 rules for their effective radius
        let width: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(50);
        let generations: usize = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(20);
        let max_radius: usize = 2; // ECAs can't have radius > 1, but let's verify

        println!("Radius survey (width={width}, gens={generations})");
        println!("Finding effective radius for all 256 rules...\n");

        use std::collections::HashMap;
        let mut radius_counts = [0usize; 3]; // count rules with effective radius 0, 1, 2+
        let mut radius_0_rules = Vec::new();
        let mut radius_gt1_rules = Vec::new();

        for rule in 0..=255u8 {
            // Generate observations
            let num_trials = 5;
            let mut transitions: Vec<(Vec<bool>, Vec<bool>)> = Vec::new();

            for trial in 0..num_trials {
                let seed: usize = trial * 12345 + 67890;
                let cells: Vec<bool> = (0..width)
                    .map(|i| ((seed.wrapping_mul(i + 1)) % 100) < 50)
                    .collect();

                let mut ca = Automaton::from_cells(cells, rule);

                for _ in 0..generations {
                    let before = ca.cells.clone();
                    ca.step();
                    let after = ca.cells.clone();
                    transitions.push((before, after));
                }
            }

            // Find minimal consistent radius
            let mut effective_radius = max_radius + 1;

            for r in 0..=max_radius {
                let window_size = 2 * r + 1;
                let mut mapping: HashMap<Vec<bool>, (usize, usize)> = HashMap::new();

                for (before, after) in &transitions {
                    let n = before.len();
                    for i in 0..n {
                        let window: Vec<bool> = (0..window_size)
                            .map(|j| {
                                let idx = (i + n - r + j) % n;
                                before[idx]
                            })
                            .collect();

                        let output = after[i];
                        let entry = mapping.entry(window).or_insert((0, 0));
                        if output {
                            entry.1 += 1;
                        } else {
                            entry.0 += 1;
                        }
                    }
                }

                let consistent = mapping.iter().all(|(_, (zeros, ones))| *zeros == 0 || *ones == 0);

                if consistent {
                    effective_radius = r;
                    break;
                }
            }

            // Categorize
            if effective_radius == 0 {
                radius_counts[0] += 1;
                radius_0_rules.push(rule);
            } else if effective_radius == 1 {
                radius_counts[1] += 1;
            } else {
                radius_counts[2] += 1;
                radius_gt1_rules.push(rule);
            }
        }

        println!("Results:");
        println!("  Effective radius 0: {} rules", radius_counts[0]);
        println!("  Effective radius 1: {} rules", radius_counts[1]);
        println!("  Effective radius >1: {} rules (unexpected!)", radius_counts[2]);

        println!("\nRules with effective radius 0 (neighbors don't matter):");
        for chunk in radius_0_rules.chunks(16) {
            let s: String = chunk.iter().map(|r| format!("{:>4}", r)).collect::<Vec<_>>().join("");
            println!("  {}", s);
        }

        if !radius_gt1_rules.is_empty() {
            println!("\nRules with effective radius >1 (unexpected for ECAs):");
            for r in &radius_gt1_rules {
                println!("  Rule {}", r);
            }
        }

        // Analyze what makes radius-0 rules special
        println!("\nAnalysis of radius-0 rules:");
        println!("These rules have output that depends only on the center cell.");
        println!("Checking: does output = f(center) for some f?");

        for &rule in &radius_0_rules {
            // Check what the rule does for center=0 and center=1
            let mut center_0_outputs = Vec::new();
            let mut center_1_outputs = Vec::new();

            for neighborhood in 0..8u8 {
                let center = (neighborhood >> 1) & 1;
                let output = (rule >> neighborhood) & 1;

                if center == 0 {
                    center_0_outputs.push(output);
                } else {
                    center_1_outputs.push(output);
                }
            }

            let f_of_0 = if center_0_outputs.iter().all(|&x| x == 0) {
                "0"
            } else if center_0_outputs.iter().all(|&x| x == 1) {
                "1"
            } else {
                "?"
            };

            let f_of_1 = if center_1_outputs.iter().all(|&x| x == 0) {
                "0"
            } else if center_1_outputs.iter().all(|&x| x == 1) {
                "1"
            } else {
                "?"
            };

            if f_of_0 != "?" && f_of_1 != "?" {
                let name = if f_of_0 == "0" && f_of_1 == "0" {
                    "constant 0"
                } else if f_of_0 == "1" && f_of_1 == "1" {
                    "constant 1"
                } else if f_of_0 == "0" && f_of_1 == "1" {
                    "identity"
                } else {
                    "NOT"
                };
                println!("  Rule {:>3}: f(0)={}, f(1)={} ({})", rule, f_of_0, f_of_1, name);
            }
        }

        return;
    }

    if args.get(1).map(|s| s.as_str()) == Some("--compress-survey") {
        // Survey all 256 rules by compression ratio
        let width: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(79);
        let generations: usize = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(200);

        println!("Compression survey (width={width}, gens={generations})");
        println!("{:>4} {:>8} {:>12}", "Rule", "Ratio", "Class");
        println!("{}", "-".repeat(28));

        let mut results: Vec<(u8, f64)> = Vec::new();

        for rule in 0..=255u8 {
            let (_, _, ratio) = compression_ratio(rule, width, generations);
            results.push((rule, ratio));
        }

        // Sort by compression ratio
        results.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

        // Classify and print
        for (rule, ratio) in &results {
            let class = if *ratio < 0.05 {
                "trivial"      // nearly empty or constant
            } else if *ratio < 0.20 {
                "periodic"     // highly repetitive
            } else if *ratio < 0.50 {
                "structured"   // has exploitable patterns
            } else if *ratio < 0.80 {
                "complex"      // some structure
            } else {
                "chaotic"      // nearly incompressible
            };

            // Only print interesting ones (not trivial)
            if *ratio >= 0.05 {
                println!("{:>4} {:>8.3} {:>12}", rule, ratio, class);
            }
        }

        // Summary
        println!("{}", "-".repeat(28));
        let trivial = results.iter().filter(|(_, r)| *r < 0.05).count();
        let periodic = results.iter().filter(|(_, r)| *r >= 0.05 && *r < 0.20).count();
        let structured = results.iter().filter(|(_, r)| *r >= 0.20 && *r < 0.50).count();
        let complex = results.iter().filter(|(_, r)| *r >= 0.50 && *r < 0.80).count();
        let chaotic = results.iter().filter(|(_, r)| *r >= 0.80).count();

        println!("Classification:");
        println!("  Trivial (<5%):     {}", trivial);
        println!("  Periodic (5-20%):  {}", periodic);
        println!("  Structured (20-50%): {}", structured);
        println!("  Complex (50-80%):  {}", complex);
        println!("  Chaotic (>80%):    {}", chaotic);

        // Most compressible and least compressible
        println!("\nMost compressible: Rule {} ({:.1}%)", results[trivial].0, results[trivial].1 * 100.0);
        println!("Least compressible: Rule {} ({:.1}%)", results.last().unwrap().0, results.last().unwrap().1 * 100.0);

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
