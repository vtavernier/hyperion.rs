//! Definition of the IdleTracker type

use std::fmt;
use std::time::Instant;

use crate::config::IdleSettings;

/// RGB LED idle tracker
pub struct IdleTracker {
    /// Duration after which the device is considered idle
    idle_settings: IdleSettings,
    /// Total change in all color components in the current pass
    total_change: f64,
    /// Number of LEDs with non-zero color value
    nonzero_color_count: usize,
    /// Instant of the last change in any LED value
    last_change: Instant,
    /// Number of passes to apply the oneshot notification for
    oneshot_pass: u32,
    /// Are we switching back to idle after a oneshot change?
    oneshot_pending: bool,
    /// Instant of the last update of the target device
    last_update: Instant,
    /// true if an update pass is running
    pass_started: bool,
    /// Current state of the tracker
    current_state: IdleState,
}

/// Current state of the tracked device
#[derive(Clone)]
pub enum IdleState {
    /// The device is actively being updated
    Active,
    /// The device is idle and turned off
    IdleBlack,
    /// The device is idle but with a solid color
    IdleColor { update_required: bool },
}

impl IdleState {
    /// Returns true if the state requires updating the target device
    pub fn should_write(&self) -> bool {
        match self {
            IdleState::Active
            | IdleState::IdleColor {
                update_required: true,
            } => true,
            _ => false,
        }
    }

    /// Returns true if the two states are different variants
    ///
    /// # Parameters
    ///
    /// * `other`: state to compare this state to
    pub fn has_changed(&self, other: &IdleState) -> bool {
        match self {
            IdleState::Active => match other {
                IdleState::Active => false,
                _ => true,
            },
            IdleState::IdleBlack => match other {
                IdleState::IdleBlack => false,
                _ => true,
            },
            IdleState::IdleColor { .. } => match other {
                IdleState::IdleColor { .. } => false,
                _ => true,
            },
        }
    }
}

impl fmt::Display for IdleState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            IdleState::Active => write!(f, "active"),
            IdleState::IdleColor { .. } => write!(f, "idle (active)"),
            IdleState::IdleBlack => write!(f, "idle (inactive)"),
        }
    }
}

impl From<IdleSettings> for IdleTracker {
    /// Create a new idle tracker
    ///
    /// # Parameters
    ///
    /// * `idle_settings`: settings for device idle modes
    fn from(idle_settings: IdleSettings) -> Self {
        let oneshot_pass = idle_settings.retries;

        Self {
            idle_settings,
            total_change: 0.0,
            nonzero_color_count: 0,
            last_change: Instant::now(),
            oneshot_pass,
            oneshot_pending: false,
            pass_started: false,
            last_update: Instant::now(),
            current_state: IdleState::Active,
        }
    }
}

impl IdleTracker {
    /// Starts a new pass
    ///
    /// This function should be called before updating LEDs in the device.
    pub fn start_pass(&mut self) {
        assert!(!self.pass_started);

        self.total_change = 0.0;
        self.nonzero_color_count = 0;

        self.pass_started = true;
    }

    /// Completes the current pass
    ///
    /// This function should be called after the LEDs have been updated.
    ///
    /// # Parameters
    ///
    /// `oneshot`: true if this is a oneshot pass, i.e. one after which the devices should be
    /// immediately considered idle. Should be false if continuous updates are expected.
    pub fn end_pass(&mut self, oneshot: bool) {
        assert!(self.pass_started);
        trace!(
            "end pass: total_change: {}, oneshot: {}",
            self.total_change,
            oneshot
        );

        // Update change values
        if self.total_change > 2.0f64.powf(-f64::from(self.idle_settings.resolution)) {
            self.last_change = Instant::now();
        }

        // Update oneshot flags
        self.oneshot_pass = self.idle_settings.retries;
        // If a non-oneshot update comes in, stop waiting for the delay to expire
        self.oneshot_pending = self.oneshot_pending && (self.oneshot_pass > 0);

        self.pass_started = false;
    }

    /// Notifies of an update on an LED color
    ///
    /// This function should be called for every LED color update. Note that this only tracks
    /// changes, but does not update the actual color.
    ///
    /// # Parameters
    ///
    /// * `current_color`: current color of the LED being updated
    /// * `new_color`: new color value for the LED
    pub fn update_color(&mut self, current_color: &palette::LinSrgb, new_color: &palette::LinSrgb) {
        let (cr, cg, cb) = current_color.into_components();
        let (nr, ng, nb) = new_color.into_components();

        // Compute color difference
        let diff = (cr - nr).abs() + (cg - ng).abs() + (cb - nb).abs();

        // Add up total color difference
        if diff > 0.0 {
            self.total_change += f64::from(diff);
        }

        // Check if everything is black
        if nr != 0.0 || ng != 0.0 || nb != 0.0 {
            self.nonzero_color_count += 1;
        }
    }

    /// Update the current state of this tracker
    ///
    /// Note that if this method returns a state that expects the device to be written to,
    /// the internal change tracker state will be updated assuming the caller does actually
    /// write to the device.
    ///
    /// # Returns
    ///
    /// * `(changed, state)`: `changed` is true if the state changed to its current value `state`.
    /// The `changed` flag does not take into account the state of `update_required` on
    /// IdleColor.
    pub fn update_state(&mut self) -> (bool, IdleState) {
        let now = Instant::now();
        let delay_expired = now - self.last_change > self.idle_settings.delay;

        let new_state =
            // Only consider idle stats if idling is enabled, and we are not waiting on a oneshot
            // update
            if self.idle_settings.enabled && self.oneshot_pass == 0 && (delay_expired || self.oneshot_pending) {
                if self.oneshot_pending && delay_expired {
                    // The delay expired so we can release the oneshot lock
                    self.oneshot_pending = false;
                }

                if self.nonzero_color_count > 0 {
                    // When a color is displayed, we only require an update every delay/2
                    // if the device needs periodic updates to stay on.
                    IdleState::IdleColor {
                        update_required: !self.idle_settings.holds
                            && (now - self.last_update) > (self.idle_settings.delay / 2),
                    }
                } else {
                    IdleState::IdleBlack
                }
            } else {
                // Do we still have oneshot retries to perform?
                if self.oneshot_pass > 0 {
                    self.oneshot_pass -= 1;

                    // Did we run out of retries or the idle delay expired?
                    if self.oneshot_pass == 0 ||
                        delay_expired {
                            if self.oneshot_pass > 0 {
                                // We should have sent all retries before the delay expires, this
                                // makes no sense otherwise
                                warn!("idle delay {} expired before {} retries were executed ({} left), please lower retries or increase delay", humantime::Duration::from(self.idle_settings.delay), self.idle_settings.retries, self.oneshot_pass);
                                self.oneshot_pass = 0;
                            }

                            // All retries sent and delay expired, switch back to idle
                            self.oneshot_pending = true;
                        }
                }

                IdleState::Active
            };

        let changed = new_state.has_changed(&self.current_state);
        self.current_state = new_state;

        // Assume the caller does indeed write to the device after this call
        if self.current_state.should_write() {
            self.notify_update();
        }

        (changed, self.current_state.clone())
    }

    /// Notify the tracker that the device has been updated
    ///
    /// This is required to evaluate if an idle device displaying colors should receive updates or
    /// not.
    fn notify_update(&mut self) {
        self.last_update = Instant::now();
    }
}