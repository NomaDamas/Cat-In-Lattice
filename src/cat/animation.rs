use std::time::{Duration, Instant};

use super::art;
use super::state::{Accessory, Mood};

/// Which animation sequence is currently playing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimationState {
    Idle,
    Happy,
    Angry,
    Eating,
    Sleeping,
}

impl AnimationState {
    /// How long each frame is displayed.
    pub fn frame_duration(&self) -> Duration {
        match self {
            AnimationState::Idle => Duration::from_millis(150),
            AnimationState::Happy => Duration::from_millis(200),
            AnimationState::Angry => Duration::from_millis(200),
            AnimationState::Eating => Duration::from_millis(250),
            AnimationState::Sleeping => Duration::from_millis(500),
        }
    }

    /// How many frames this animation has.
    pub fn frame_count(&self) -> usize {
        match self {
            AnimationState::Idle => art::IDLE_FRAMES.len(),
            // Single-frame states just hold the pose
            AnimationState::Happy => 1,
            AnimationState::Angry => 1,
            AnimationState::Eating => 1,
            AnimationState::Sleeping => 1,
        }
    }

    /// Whether this animation loops or plays once and holds.
    pub fn loops(&self) -> bool {
        match self {
            AnimationState::Idle => true,
            AnimationState::Happy => false,
            AnimationState::Angry => false,
            AnimationState::Eating => false,
            AnimationState::Sleeping => false,
        }
    }
}

/// Controls frame sequencing, timing, and transitions.
pub struct AnimationController {
    /// Current animation being played.
    state: AnimationState,
    /// Current frame index within the animation.
    frame_index: usize,
    /// When the current frame started displaying.
    frame_start: Instant,
    /// How long to hold a non-looping animation before returning to idle.
    hold_duration: Duration,
    /// When the current non-looping animation started.
    state_start: Instant,
}

impl AnimationController {
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            state: AnimationState::Idle,
            frame_index: 0,
            frame_start: now,
            hold_duration: Duration::from_secs(2),
            state_start: now,
        }
    }

    /// Transition to a new animation state.
    pub fn set_state(&mut self, new_state: AnimationState) {
        if self.state != new_state {
            let now = Instant::now();
            self.state = new_state;
            self.frame_index = 0;
            self.frame_start = now;
            self.state_start = now;
        }
    }

    /// Set how long non-looping animations hold before returning to idle.
    pub fn set_hold_duration(&mut self, duration: Duration) {
        self.hold_duration = duration;
    }

    /// Derive animation state from the cat's current mood.
    pub fn set_from_mood(&mut self, mood: Mood) {
        let target = match mood {
            Mood::Happy => AnimationState::Happy,
            Mood::Neutral => AnimationState::Idle,
            Mood::Sad => AnimationState::Idle, // sad uses idle frames (mood colors differ)
            Mood::Angry => AnimationState::Angry,
            Mood::Sleeping => AnimationState::Sleeping,
        };
        self.set_state(target);
    }

    /// Trigger the eating animation (temporarily overrides mood-based state).
    pub fn play_eating(&mut self) {
        self.set_state(AnimationState::Eating);
    }

    /// Trigger the happy animation.
    pub fn play_happy(&mut self) {
        self.set_state(AnimationState::Happy);
    }

    /// Trigger the angry animation.
    pub fn play_angry(&mut self) {
        self.set_state(AnimationState::Angry);
    }

    /// Advance the animation clock. Call this every tick.
    /// Returns true if the frame changed (so the caller knows to re-render).
    pub fn tick(&mut self) -> bool {
        let now = Instant::now();

        // If we're in a non-looping state that has expired, return to idle
        if !self.state.loops() && now.duration_since(self.state_start) >= self.hold_duration {
            self.set_state(AnimationState::Idle);
            return true;
        }

        // Check if it's time to advance to the next frame
        let elapsed = now.duration_since(self.frame_start);
        if elapsed >= self.state.frame_duration() {
            let count = self.state.frame_count();
            if count > 1 {
                self.frame_index = if self.state.loops() {
                    (self.frame_index + 1) % count
                } else {
                    (self.frame_index + 1).min(count - 1)
                };
            }
            self.frame_start = now;
            return true;
        }

        false
    }

    /// Get the raw art frame for the current animation state and frame index.
    pub fn current_frame(&self) -> &'static [&'static str] {
        match self.state {
            AnimationState::Idle => {
                art::IDLE_FRAMES[self.frame_index % art::IDLE_FRAMES.len()]
            }
            AnimationState::Happy => art::HAPPY,
            AnimationState::Angry => art::ANGRY,
            AnimationState::Eating => art::EATING,
            AnimationState::Sleeping => art::SLEEPING,
        }
    }

    /// Get the current frame with accessories composited on top.
    pub fn current_frame_with_accessories(&self, accessories: &[Accessory]) -> Vec<String> {
        let base = self.current_frame();
        let mut result: Vec<String> = base.iter().map(|s| s.to_string()).collect();

        for acc in accessories {
            let overlay = art::overlay_for(acc);
            result = art::composite(
                // Convert Vec<String> to slice of &str for composite
                &result.iter().map(|s| s.as_str()).collect::<Vec<&str>>(),
                overlay,
            );
        }

        result
    }

    /// Current animation state.
    pub fn state(&self) -> AnimationState {
        self.state
    }

    /// Current frame index.
    pub fn frame_index(&self) -> usize {
        self.frame_index
    }
}

impl Default for AnimationController {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_state() {
        let ctrl = AnimationController::new();
        assert_eq!(ctrl.state(), AnimationState::Idle);
        assert_eq!(ctrl.frame_index(), 0);
    }

    #[test]
    fn test_transition() {
        let mut ctrl = AnimationController::new();
        ctrl.set_state(AnimationState::Happy);
        assert_eq!(ctrl.state(), AnimationState::Happy);
        assert_eq!(ctrl.frame_index(), 0);
    }

    #[test]
    fn test_current_frame_returns_valid_art() {
        let ctrl = AnimationController::new();
        let frame = ctrl.current_frame();
        assert!(!frame.is_empty());
        assert!(frame.len() >= 10); // cat should be at least 10 rows
    }

    #[test]
    fn test_frame_with_accessories() {
        let ctrl = AnimationController::new();
        let frame = ctrl.current_frame_with_accessories(&[Accessory::Hat]);
        assert!(!frame.is_empty());
    }

    #[test]
    fn test_idle_loops() {
        assert!(AnimationState::Idle.loops());
        assert!(!AnimationState::Happy.loops());
    }
}
