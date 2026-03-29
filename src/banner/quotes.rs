use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

const QUOTES_JSON: &str = include_str!("../../data/quotes.json");
const ROTATION_INTERVAL: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Quote {
    pub text: String,
    pub author: String,
}

impl std::fmt::Display for Quote {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "\"{}\" — {}", self.text, self.author)
    }
}

pub struct QuoteRotator {
    quotes: Vec<Quote>,
    current_index: usize,
    last_rotation: Instant,
}

impl QuoteRotator {
    pub fn new() -> Self {
        let quotes: Vec<Quote> =
            serde_json::from_str(QUOTES_JSON).expect("bundled quotes.json must be valid");
        assert!(
            !quotes.is_empty(),
            "quotes.json must contain at least one quote"
        );

        let mut rng = rand::thread_rng();
        let initial_index = (0..quotes.len())
            .collect::<Vec<_>>()
            .choose(&mut rng)
            .copied()
            .unwrap_or(0);

        Self {
            quotes,
            current_index: initial_index,
            last_rotation: Instant::now(),
        }
    }

    /// Returns the current quote, rotating to a new random one if the interval has elapsed.
    pub fn current(&mut self) -> &Quote {
        if self.last_rotation.elapsed() >= ROTATION_INTERVAL {
            self.rotate();
        }
        &self.quotes[self.current_index]
    }

    /// Force rotation to a new random quote (different from current if possible).
    pub fn rotate(&mut self) {
        let mut rng = rand::thread_rng();
        if self.quotes.len() > 1 {
            let old = self.current_index;
            while self.current_index == old {
                self.current_index = *((0..self.quotes.len())
                    .collect::<Vec<_>>()
                    .choose(&mut rng)
                    .unwrap());
            }
        }
        self.last_rotation = Instant::now();
    }

    /// Returns the total number of loaded quotes.
    pub fn len(&self) -> usize {
        self.quotes.len()
    }

    /// Returns true if no quotes are loaded (should never happen with bundled data).
    pub fn is_empty(&self) -> bool {
        self.quotes.is_empty()
    }

    /// Check whether a rotation is due without actually rotating.
    pub fn needs_rotation(&self) -> bool {
        self.last_rotation.elapsed() >= ROTATION_INTERVAL
    }
}

impl Default for QuoteRotator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_bundled_quotes() {
        let rotator = QuoteRotator::new();
        assert!(rotator.len() >= 50);
    }

    #[test]
    fn current_returns_a_quote() {
        let mut rotator = QuoteRotator::new();
        let q = rotator.current();
        assert!(!q.text.is_empty());
        assert!(!q.author.is_empty());
    }

    #[test]
    fn rotate_changes_quote() {
        let mut rotator = QuoteRotator::new();
        let first = rotator.current_index;
        // With 50+ quotes the probability of picking the same one is tiny,
        // but we try a few times to be safe.
        let mut changed = false;
        for _ in 0..20 {
            rotator.rotate();
            if rotator.current_index != first {
                changed = true;
                break;
            }
        }
        assert!(changed, "rotate should pick a different quote");
    }
}
