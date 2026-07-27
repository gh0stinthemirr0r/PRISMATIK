//! Emergency stop: cancel-all, flatten, halt automation (`P7-SS-02` surface).

use serde::{Deserialize, Serialize};

/// Operator emergency-stop latch.
///
/// Engaging sets all three flags. Clearing requires an explicit reset after
/// the runbook completes — flags are independent for partial recovery tests.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmergencyStop {
    /// Cancel all working orders at every connected gateway.
    pub cancel_all: bool,
    /// Flatten open positions to flat (marketable exit intents).
    pub flatten: bool,
    /// Halt strategy / automation admission of new intents.
    pub halt_automation: bool,
}

impl EmergencyStop {
    /// All flags clear (normal operation).
    pub fn clear() -> Self {
        Self::default()
    }

    /// Engage the full emergency stop (all three flags).
    pub fn engage(&mut self) {
        self.cancel_all = true;
        self.flatten = true;
        self.halt_automation = true;
    }

    /// True when automation must not admit new intents.
    pub fn is_halted(&self) -> bool {
        self.halt_automation
    }

    /// True when any emergency action is still pending.
    pub fn is_engaged(&self) -> bool {
        self.cancel_all || self.flatten || self.halt_automation
    }

    /// Reset after runbook completion.
    pub fn reset(&mut self) {
        *self = Self::clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engage_sets_cancel_flatten_and_halt() {
        let mut stop = EmergencyStop::clear();
        assert!(!stop.is_engaged());
        assert!(!stop.is_halted());

        stop.engage();
        assert!(stop.cancel_all);
        assert!(stop.flatten);
        assert!(stop.halt_automation);
        assert!(stop.is_halted());
        assert!(stop.is_engaged());

        stop.reset();
        assert!(!stop.is_engaged());
    }
}
