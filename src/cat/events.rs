use chrono::{DateTime, Datelike, Duration, Utc};
use rand::Rng;
use serde::{Deserialize, Serialize};

use super::state::{Accessory, CatState};

/// The kind of event that can occur.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventType {
    /// Cat wants to be petted.
    Pet,
    /// Cat wants to be fed.
    Feed,
    /// Special event that awards an accessory.
    Special(Accessory),
}

/// A scheduled cat event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatEvent {
    /// What kind of event this is.
    pub event_type: EventType,
    /// When the event fires.
    pub scheduled_at: DateTime<Utc>,
    /// How long the player has to respond (seconds).
    pub window_secs: u64,
    /// Whether the event has been resolved.
    pub resolved: bool,
    /// Whether the player responded in time.
    pub success: Option<bool>,
}

impl CatEvent {
    /// Create a new unresolved event.
    pub fn new(event_type: EventType, scheduled_at: DateTime<Utc>) -> Self {
        Self {
            event_type,
            scheduled_at,
            window_secs: 10,
            resolved: false,
            success: None,
        }
    }

    /// Whether this event is currently active (within its response window).
    pub fn is_active(&self, now: DateTime<Utc>) -> bool {
        if self.resolved {
            return false;
        }
        let elapsed = now.signed_duration_since(self.scheduled_at);
        elapsed >= Duration::zero() && elapsed < Duration::seconds(self.window_secs as i64)
    }

    /// Whether this event's window has expired without response.
    pub fn is_expired(&self, now: DateTime<Utc>) -> bool {
        if self.resolved {
            return false;
        }
        let elapsed = now.signed_duration_since(self.scheduled_at);
        elapsed >= Duration::seconds(self.window_secs as i64)
    }

    /// Mark this event as responded to (success).
    pub fn respond_success(&mut self) {
        self.resolved = true;
        self.success = Some(true);
    }

    /// Mark this event as missed (failure).
    pub fn respond_failure(&mut self) {
        self.resolved = true;
        self.success = Some(false);
    }
}

/// Manages the event schedule and tracks statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventScheduler {
    /// All events for the current day.
    pub events: Vec<CatEvent>,
    /// The date these events were generated for (year, ordinal day).
    generated_for_day: Option<(i32, u32)>,
    /// Successful event responses.
    pub successes: u64,
    /// Missed events.
    pub failures: u64,
    /// Active hours start (inclusive, 0-23).
    pub active_hour_start: u8,
    /// Active hours end (exclusive, 0-23). If end < start, wraps midnight.
    pub active_hour_end: u8,
    /// Number of regular events per day.
    pub events_per_day: u32,
    /// Number of special (accessory) events per day when eligible.
    pub special_events_per_day: u32,
}

impl Default for EventScheduler {
    fn default() -> Self {
        Self {
            events: Vec::new(),
            generated_for_day: None,
            successes: 0,
            failures: 0,
            active_hour_start: 8,  // 8 AM
            active_hour_end: 23,   // 11 PM
            events_per_day: 20,
            special_events_per_day: 3,
        }
    }
}

impl EventScheduler {
    pub fn new() -> Self {
        Self::default()
    }

    /// How many active hours in a day.
    fn active_hours(&self) -> u32 {
        if self.active_hour_end > self.active_hour_start {
            (self.active_hour_end - self.active_hour_start) as u32
        } else {
            (24 - self.active_hour_start as u32) + self.active_hour_end as u32
        }
    }

    /// Check if a given hour is within active hours.
    #[allow(dead_code)]
    fn is_active_hour(&self, hour: u32) -> bool {
        if self.active_hour_end > self.active_hour_start {
            hour >= self.active_hour_start as u32 && hour < self.active_hour_end as u32
        } else {
            hour >= self.active_hour_start as u32 || hour < self.active_hour_end as u32
        }
    }

    /// Generate events for today if not already generated.
    /// `cat` is used to determine if special events should be included.
    pub fn ensure_today_events(&mut self, cat: &CatState) {
        let now = Utc::now();
        let today = (now.date_naive().year_ce().1 as i32, now.ordinal());

        if self.generated_for_day == Some(today) {
            return;
        }

        self.events.clear();
        self.generated_for_day = Some(today);

        let mut rng = rand::thread_rng();
        let active_mins = self.active_hours() * 60;

        // Generate regular events (mix of Pet and Feed)
        for _ in 0..self.events_per_day {
            let offset_mins = rng.gen_range(0..active_mins);
            let event_time = self.active_start_today(now) + Duration::minutes(offset_mins as i64);

            let event_type = if rng.gen_bool(0.6) {
                EventType::Pet
            } else {
                EventType::Feed
            };

            self.events.push(CatEvent::new(event_type, event_time));
        }

        // Generate special accessory events if affinity is high enough
        if cat.can_unlock_accessory() {
            if let Some(next_acc) = cat.next_unlockable_accessory() {
                for _ in 0..self.special_events_per_day {
                    let offset_mins = rng.gen_range(0..active_mins);
                    let event_time =
                        self.active_start_today(now) + Duration::minutes(offset_mins as i64);
                    self.events
                        .push(CatEvent::new(EventType::Special(next_acc), event_time));
                }
            }
        }

        // Sort by scheduled time
        self.events
            .sort_by(|a, b| a.scheduled_at.cmp(&b.scheduled_at));
    }

    /// Get the DateTime for the start of active hours today.
    fn active_start_today(&self, now: DateTime<Utc>) -> DateTime<Utc> {
        now.date_naive()
            .and_hms_opt(self.active_hour_start as u32, 0, 0)
            .expect("valid time")
            .and_utc()
    }

    /// Find the first currently active (unresolved, within window) event.
    pub fn active_event(&self) -> Option<&CatEvent> {
        let now = Utc::now();
        self.events.iter().find(|e| e.is_active(now))
    }

    /// Find the index of the first currently active event.
    pub fn active_event_index(&self) -> Option<usize> {
        let now = Utc::now();
        self.events.iter().position(|e| e.is_active(now))
    }

    /// Process all expired events: mark them as failures and return how many were missed.
    pub fn process_expired(&mut self, cat: &mut CatState) -> u32 {
        let now = Utc::now();
        let mut missed = 0u32;

        for event in &mut self.events {
            if event.is_expired(now) {
                event.respond_failure();
                cat.miss_event();
                missed += 1;
            }
        }
        self.failures += missed as u64;
        missed
    }

    /// Respond to the currently active event. Returns the event type if successful.
    pub fn respond_to_active(&mut self, cat: &mut CatState) -> Option<EventType> {
        let now = Utc::now();
        let idx = self.events.iter().position(|e| e.is_active(now))?;

        let event = &mut self.events[idx];
        event.respond_success();
        self.successes += 1;

        let event_type = event.event_type.clone();

        match &event_type {
            EventType::Pet => cat.pet(),
            EventType::Feed => cat.feed(),
            EventType::Special(acc) => {
                cat.pet();
                cat.add_accessory(*acc);
            }
        }

        Some(event_type)
    }

    /// Time until the next unresolved event, if any.
    pub fn time_until_next(&self) -> Option<Duration> {
        let now = Utc::now();
        self.events
            .iter()
            .filter(|e| !e.resolved && e.scheduled_at > now)
            .map(|e| e.scheduled_at.signed_duration_since(now))
            .min()
    }

    /// Count of events remaining today (unresolved).
    pub fn remaining_count(&self) -> usize {
        self.events.iter().filter(|e| !e.resolved).count()
    }

    /// Success rate as a percentage (0.0 - 100.0).
    pub fn success_rate(&self) -> f64 {
        let total = self.successes + self.failures;
        if total == 0 {
            return 100.0;
        }
        (self.successes as f64 / total as f64) * 100.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_lifecycle() {
        let now = Utc::now();
        let mut event = CatEvent::new(EventType::Pet, now);

        assert!(event.is_active(now));
        assert!(!event.is_expired(now));
        assert!(!event.resolved);

        event.respond_success();
        assert!(event.resolved);
        assert_eq!(event.success, Some(true));
        assert!(!event.is_active(now));
    }

    #[test]
    fn test_event_expiry() {
        let past = Utc::now() - Duration::seconds(15);
        let event = CatEvent::new(EventType::Feed, past);
        let now = Utc::now();

        assert!(!event.is_active(now));
        assert!(event.is_expired(now));
    }

    #[test]
    fn test_scheduler_generates_events() {
        let mut scheduler = EventScheduler::new();
        let cat = CatState::new();
        scheduler.ensure_today_events(&cat);

        // Should have at least events_per_day events
        assert!(scheduler.events.len() >= scheduler.events_per_day as usize);
    }

    #[test]
    fn test_scheduler_idempotent() {
        let mut scheduler = EventScheduler::new();
        let cat = CatState::new();
        scheduler.ensure_today_events(&cat);
        let count = scheduler.events.len();
        scheduler.ensure_today_events(&cat);
        assert_eq!(scheduler.events.len(), count);
    }

    #[test]
    fn test_special_events_with_high_affinity() {
        let mut scheduler = EventScheduler::new();
        let mut cat = CatState::new();
        cat.affinity = 80.0; // above threshold
        scheduler.ensure_today_events(&cat);

        let specials = scheduler
            .events
            .iter()
            .filter(|e| matches!(e.event_type, EventType::Special(_)))
            .count();
        assert_eq!(specials, scheduler.special_events_per_day as usize);
    }

    #[test]
    fn test_success_rate() {
        let mut scheduler = EventScheduler::new();
        scheduler.successes = 8;
        scheduler.failures = 2;
        assert!((scheduler.success_rate() - 80.0).abs() < f64::EPSILON);
    }
}
