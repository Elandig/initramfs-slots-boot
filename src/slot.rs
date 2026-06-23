//! The reels. There's no money and no paytable - the only thing that matters is
//! whether the payline reads 7-7-7. Everything else is just fruit to spin past.

use crate::rng::Rng;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Symbol {
    Seven,
    Dollar,
    Heart,
    Diamond,
    Club,
    Spade,
}

impl Symbol {
    /// What shows on screen. Card suits, `7` and `$` - all single-width and all part
    /// of the classic console font, so they render the same in QEMU, on a bare VGA
    /// console, and in a terminal. (Emoji do not - they come out as garbage on a
    /// console font, which is exactly what we're avoiding.)
    pub fn glyph(self) -> &'static str {
        match self {
            Symbol::Seven => "7",
            Symbol::Dollar => "$",
            Symbol::Heart => "\u{2665}",   // ♥
            Symbol::Diamond => "\u{2666}", // ♦
            Symbol::Club => "\u{2663}",    // ♣
            Symbol::Spade => "\u{2660}",   // ♠
        }
    }

    /// ANSI colour for the glyph. These are plain text, so colour actually sticks.
    pub fn color(self) -> &'static str {
        match self {
            Symbol::Seven => "\x1b[1;93m",   // bright yellow
            Symbol::Dollar => "\x1b[1;92m",  // bright green
            Symbol::Heart => "\x1b[1;91m",   // red
            Symbol::Diamond => "\x1b[1;91m", // red
            Symbol::Club => "\x1b[1;96m",    // cyan
            Symbol::Spade => "\x1b[1;97m",   // white
        }
    }

    pub fn is_seven(self) -> bool {
        matches!(self, Symbol::Seven)
    }
}

pub struct Machine {
    reels: [Vec<Symbol>; 3],
}

impl Default for Machine {
    fn default() -> Self {
        Self::new()
    }
}

impl Machine {
    pub fn new() -> Self {
        use Symbol::*;
        // Plenty of fruit, a handful of sevens. The right reel carries one fewer
        // seven than the others, so 7-7-and-a-miss turns up a lot - that's the
        // near-miss that keeps you going. Works out to roughly 1 in 140 a spin.
        let left = build(&[
            (Seven, 5),
            (Dollar, 4),
            (Heart, 4),
            (Diamond, 4),
            (Club, 4),
            (Spade, 3),
        ]);
        let middle = left.clone();
        let right = build(&[
            (Seven, 4),
            (Dollar, 4),
            (Heart, 4),
            (Diamond, 4),
            (Club, 4),
            (Spade, 4),
        ]);
        Machine {
            reels: [left, middle, right],
        }
    }

    pub fn reel_len(&self, reel: usize) -> usize {
        self.reels[reel].len()
    }

    pub fn symbol(&self, reel: usize, pos: usize) -> Symbol {
        let r = &self.reels[reel];
        r[pos % r.len()]
    }

    pub fn random_stop(&self, reel: usize, rng: &mut Rng) -> usize {
        rng.below(self.reels[reel].len())
    }

    /// One full spin: a random stop on each reel, returned as the payline.
    pub fn spin(&self, rng: &mut Rng) -> [Symbol; 3] {
        [
            self.symbol(0, self.random_stop(0, rng)),
            self.symbol(1, self.random_stop(1, rng)),
            self.symbol(2, self.random_stop(2, rng)),
        ]
    }

    pub fn is_jackpot(line: [Symbol; 3]) -> bool {
        line.iter().all(|s| s.is_seven())
    }

    /// Probability of 7-7-7 on a single spin. Only used by the tests.
    #[cfg(test)]
    pub fn jackpot_chance(&self) -> f64 {
        self.reels
            .iter()
            .map(|r| r.iter().filter(|s| s.is_seven()).count() as f64 / r.len() as f64)
            .product()
    }
}

// Lay the symbols out round-robin rather than in blocks, so a real reel strip's worth
// of variety scrolls past while spinning (and you don't get a teasing run of sevens).
// The multiset - and therefore the odds - is exactly the same either way.
fn build(counts: &[(Symbol, usize)]) -> Vec<Symbol> {
    let mut left: Vec<(Symbol, usize)> = counts.to_vec();
    let mut strip = Vec::new();
    let mut placed = true;
    while placed {
        placed = false;
        for (sym, n) in left.iter_mut() {
            if *n > 0 {
                strip.push(*sym);
                *n -= 1;
                placed = true;
            }
        }
    }
    strip
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_three_sevens_is_a_jackpot() {
        assert!(Machine::is_jackpot([Symbol::Seven; 3]));
        assert!(!Machine::is_jackpot([
            Symbol::Seven,
            Symbol::Seven,
            Symbol::Heart
        ]));
        assert!(!Machine::is_jackpot([Symbol::Heart; 3]));
    }

    #[test]
    fn jackpot_is_rare_but_possible() {
        let p = Machine::new().jackpot_chance();
        assert!(p > 0.0 && p < 0.05, "odds out of range: {p}");
    }

    #[test]
    fn a_fixed_seed_gets_there_eventually() {
        let m = Machine::new();
        let mut rng = Rng::new(1);
        let hit = (0..100_000).any(|_| Machine::is_jackpot(m.spin(&mut rng)));
        assert!(hit);
    }
}
