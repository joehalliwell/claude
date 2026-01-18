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

use std::fmt;

#[derive(Clone)]
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
