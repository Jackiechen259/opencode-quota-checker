use crate::state::UiError;
use volc_core::{AlertDecision, VolcError};

/// Delivers threshold decisions using the native notification service.
pub fn deliver(decisions: Vec<AlertDecision>) -> Result<(), UiError> {
    for decision in decisions {
        notify_rust::Notification::new()
            .summary(&decision.title)
            .body(&decision.body)
            .show()
            .map_err(|error| {
                UiError::from(VolcError::Config(format!(
                    "desktop notification failed: {error}"
                )))
            })?;
    }
    Ok(())
}
