//! Profile State Transition and Convergence Controller.
//!
//! Coordinates idempotent profile state operations (Enable, Disable, Delete)
//! with independent verification reads, preventing ambiguous card states
//! across modem resets or channel interruptions.

use std::fmt;

/// Result of an executed profile state transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProfileStateTransitionResult {
    pub command_attempts: usize,
    pub verification_attempts: usize,
    pub actual_enabled: bool,
}

/// Errors arising during profile state transition and verification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StateTransitionError {
    NonConvergence {
        target_enabled: bool,
        command_attempts: usize,
        verification_attempts: usize,
        actual_enabled: Option<bool>,
        last_error: Option<String>,
    },
    Fatal(String),
}

impl fmt::Display for StateTransitionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonConvergence {
                target_enabled,
                command_attempts,
                verification_attempts,
                actual_enabled,
                last_error,
            } => {
                let observed = match actual_enabled {
                    Some(en) => format!("enabled={en}"),
                    None => "profile absent or unreadable".to_string(),
                };
                let cause = match last_error {
                    Some(err) => format!(", last error: {err}"),
                    None => String::new(),
                };
                write!(
                    f,
                    "Profile state did not converge to enabled={target_enabled} after \
                     {command_attempts} command attempt(s) and {verification_attempts} \
                     verification attempt(s); last observed: {observed}{cause}"
                )
            }
            Self::Fatal(msg) => write!(f, "Fatal state transition error: {msg}"),
        }
    }
}

impl std::error::Error for StateTransitionError {}

/// Pure profile-state convergence evaluator.
///
/// This does not submit commands, reconnect, delay, or classify runtime errors;
/// those effects belong to the adapter-owning caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProfileStateTransition {
    pub max_command_attempts: usize,
    pub max_verification_attempts: usize,
    pub max_transient_failures_without_read: usize,
}

impl Default for ProfileStateTransition {
    fn default() -> Self {
        Self {
            max_command_attempts: 2,
            max_verification_attempts: 6,
            max_transient_failures_without_read: 2,
        }
    }
}

impl ProfileStateTransition {
    #[must_use]
    pub const fn new(
        max_command_attempts: usize,
        max_verification_attempts: usize,
        max_transient_failures_without_read: usize,
    ) -> Self {
        Self {
            max_command_attempts,
            max_verification_attempts,
            max_transient_failures_without_read,
        }
    }

    /// Evaluates convergence given a sequence of verification reads.
    ///
    /// A successful read with `None` counts as a responsive read but cannot
    /// satisfy an enable/disable request.
    pub fn evaluate_convergence(
        &self,
        target_enabled: bool,
        command_attempt: usize,
        verification_reads: &[Result<Option<bool>, String>],
    ) -> Result<ProfileStateTransitionResult, StateTransitionError> {
        let mut verification_attempts = 0usize;
        let mut successful_reads = 0usize;
        let mut transient_failures = 0usize;
        let mut last_observed: Option<bool> = None;
        let mut last_error: Option<String> = None;

        for res in verification_reads {
            verification_attempts += 1;
            match res {
                Ok(Some(actual)) => {
                    successful_reads += 1;
                    last_observed = Some(*actual);
                    if *actual == target_enabled {
                        return Ok(ProfileStateTransitionResult {
                            command_attempts: command_attempt,
                            verification_attempts,
                            actual_enabled: *actual,
                        });
                    }
                }
                Ok(None) => {
                    successful_reads += 1;
                    last_observed = None;
                }
                Err(err) => {
                    transient_failures += 1;
                    last_error = Some(err.clone());
                    if successful_reads == 0
                        && transient_failures >= self.max_transient_failures_without_read
                    {
                        return Err(StateTransitionError::NonConvergence {
                            target_enabled,
                            command_attempts: command_attempt,
                            verification_attempts,
                            actual_enabled: last_observed,
                            last_error,
                        });
                    }
                }
            }
        }

        Err(StateTransitionError::NonConvergence {
            target_enabled,
            command_attempts: command_attempt,
            verification_attempts,
            actual_enabled: last_observed,
            last_error,
        })
    }
}
