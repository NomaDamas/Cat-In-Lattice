use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

/// The cat's emotional state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Mood {
    Happy,
    Neutral,
    Sad,
    Angry,
    Sleeping,
}

/// Unlockable accessories the cat can wear.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Accessory {
    Hat,
    Bow,
    Glasses,
    Scarf,
}

impl Accessory {
    /// All possible accessories in unlock order.
    pub const ALL: &'static [Accessory] = &[
        Accessory::Bow,
        Accessory::Glasses,
        Accessory::Hat,
        Accessory::Scarf,
    ];
}

/// Persistent cat state, serializable to JSON for save/load.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatState {
    /// Affinity / bond level: 0.0 to 100.0
    pub affinity: f64,
    /// Hunger level: 0.0 (full) to 100.0 (starving)
    pub hunger: f64,
    /// Current mood (derived from affinity + hunger, or manually set)
    pub mood: Mood,
    /// Equipped accessories
    pub accessories: Vec<Accessory>,
    /// When the cat was last fed
    pub last_fed: DateTime<Utc>,
    /// When the cat was last petted
    pub last_petted: DateTime<Utc>,
    /// Lifetime pet count
    pub total_pets: u64,
    /// Lifetime feed count
    pub total_feeds: u64,
    /// When this cat was created
    pub created_at: DateTime<Utc>,
}

impl Default for CatState {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            affinity: 50.0,
            hunger: 30.0,
            mood: Mood::Neutral,
            accessories: Vec::new(),
            last_fed: now,
            last_petted: now,
            total_pets: 0,
            total_feeds: 0,
            created_at: now,
        }
    }
}

impl CatState {
    /// Create a brand-new cat.
    pub fn new() -> Self {
        Self::default()
    }

    /// Pet the cat: raises affinity, resets last_petted.
    pub fn pet(&mut self) {
        self.last_petted = Utc::now();
        self.total_pets += 1;
        self.affinity = (self.affinity + 3.0).min(100.0);
        self.recalculate_mood();
    }

    /// Feed the cat: reduces hunger, slight affinity boost.
    pub fn feed(&mut self) {
        self.last_fed = Utc::now();
        self.total_feeds += 1;
        self.hunger = (self.hunger - 25.0).max(0.0);
        self.affinity = (self.affinity + 1.5).min(100.0);
        self.recalculate_mood();
    }

    /// Called when a scheduled event is missed — cat gets upset.
    pub fn miss_event(&mut self) {
        self.affinity = (self.affinity - 5.0).max(0.0);
        self.mood = Mood::Angry;
    }

    /// Add an accessory (no duplicates).
    pub fn add_accessory(&mut self, acc: Accessory) {
        if !self.accessories.contains(&acc) {
            self.accessories.push(acc);
        }
    }

    /// Remove an accessory.
    pub fn remove_accessory(&mut self, acc: &Accessory) {
        self.accessories.retain(|a| a != acc);
    }

    /// Simulate hunger increasing over elapsed time.
    /// Call this on each tick. Hunger grows ~4.2 per hour (reaches 100 in ~24h).
    pub fn tick_hunger(&mut self) {
        let now = Utc::now();
        let elapsed = now.signed_duration_since(self.last_fed);
        let hours = elapsed.num_seconds() as f64 / 3600.0;
        // Asymptotic curve so it slows down approaching 100
        self.hunger = 100.0 * (1.0 - (-0.042 * hours).exp());
        self.hunger = self.hunger.clamp(0.0, 100.0);
        self.recalculate_mood();
    }

    /// Affinity decays slowly if you haven't interacted in a while.
    /// Loses ~1 point per hour of no petting, down to a floor of 10.
    pub fn tick_affinity_decay(&mut self) {
        let now = Utc::now();
        let since_pet = now.signed_duration_since(self.last_petted);
        let hours_idle = (since_pet.num_seconds() as f64 / 3600.0).max(0.0);
        if hours_idle > 1.0 {
            let decay = (hours_idle - 1.0) * 1.0; // 1 pt per hour after first hour
            self.affinity = (self.affinity - decay * 0.01).max(10.0); // per-tick fraction
        }
        self.recalculate_mood();
    }

    /// Derive mood from current affinity and hunger.
    pub fn recalculate_mood(&mut self) {
        // Don't override sleeping — that's set explicitly
        if self.mood == Mood::Sleeping {
            return;
        }
        // Don't override angry immediately — let it persist briefly
        if self.mood == Mood::Angry {
            // Angry fades after 30 seconds
            let since_miss = Utc::now().signed_duration_since(self.last_petted);
            if since_miss < Duration::seconds(30) {
                return;
            }
        }

        self.mood = Self::compute_mood(self.affinity, self.hunger);
    }

    /// Pure mood calculation from stats.
    pub fn compute_mood(affinity: f64, hunger: f64) -> Mood {
        let score = affinity - hunger * 0.5;
        if score >= 60.0 {
            Mood::Happy
        } else if score >= 25.0 {
            Mood::Neutral
        } else {
            Mood::Sad
        }
    }

    /// Put the cat to sleep.
    pub fn sleep(&mut self) {
        self.mood = Mood::Sleeping;
    }

    /// Wake the cat up (mood recalculates).
    pub fn wake(&mut self) {
        self.mood = Mood::Neutral;
        self.recalculate_mood();
    }

    /// Whether the cat is eligible for special accessory events.
    pub fn can_unlock_accessory(&self) -> bool {
        self.affinity >= 70.0
    }

    /// Return the next accessory the cat hasn't unlocked yet, if any.
    pub fn next_unlockable_accessory(&self) -> Option<Accessory> {
        Accessory::ALL
            .iter()
            .find(|a| !self.accessories.contains(a))
            .copied()
    }

    /// Serialize to JSON string.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Deserialize from JSON string.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_state() {
        let cat = CatState::new();
        assert!((cat.affinity - 50.0).abs() < f64::EPSILON);
        assert_eq!(cat.mood, Mood::Neutral);
        assert!(cat.accessories.is_empty());
    }

    #[test]
    fn test_pet_increases_affinity() {
        let mut cat = CatState::new();
        let before = cat.affinity;
        cat.pet();
        assert!(cat.affinity > before);
        assert_eq!(cat.total_pets, 1);
    }

    #[test]
    fn test_feed_reduces_hunger() {
        let mut cat = CatState::new();
        cat.hunger = 60.0;
        cat.feed();
        assert!((cat.hunger - 35.0).abs() < f64::EPSILON);
        assert_eq!(cat.total_feeds, 1);
    }

    #[test]
    fn test_mood_computation() {
        assert_eq!(CatState::compute_mood(100.0, 0.0), Mood::Happy);
        assert_eq!(CatState::compute_mood(50.0, 30.0), Mood::Neutral);
        assert_eq!(CatState::compute_mood(10.0, 90.0), Mood::Sad);
    }

    #[test]
    fn test_accessory_no_duplicates() {
        let mut cat = CatState::new();
        cat.add_accessory(Accessory::Hat);
        cat.add_accessory(Accessory::Hat);
        assert_eq!(cat.accessories.len(), 1);
    }

    #[test]
    fn test_serialization_roundtrip() {
        let cat = CatState::new();
        let json = cat.to_json().unwrap();
        let restored = CatState::from_json(&json).unwrap();
        assert!((cat.affinity - restored.affinity).abs() < f64::EPSILON);
        assert_eq!(cat.mood, restored.mood);
    }
}
