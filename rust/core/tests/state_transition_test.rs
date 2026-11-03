use nlpa_core::{ProfileStateTransition, ProfileStateTransitionResult, StateTransitionError};

#[test]
fn test_state_transition_immediate_convergence() {
    let transition = ProfileStateTransition::default();
    let reads = vec![Ok(Some(true))];

    let result = transition
        .evaluate_convergence(true, 1, &reads)
        .expect("immediate convergence");

    assert_eq!(
        result,
        ProfileStateTransitionResult {
            command_attempts: 1,
            verification_attempts: 1,
            actual_enabled: true,
        }
    );
}

#[test]
fn test_state_transition_delayed_convergence() {
    let transition = ProfileStateTransition::default();
    // Simulate modem reset delay where card first reports old state then converges
    let reads = vec![Ok(Some(false)), Ok(Some(false)), Ok(Some(true))];

    let result = transition
        .evaluate_convergence(true, 1, &reads)
        .expect("eventual convergence");

    assert_eq!(
        result,
        ProfileStateTransitionResult {
            command_attempts: 1,
            verification_attempts: 3,
            actual_enabled: true,
        }
    );
}

#[test]
fn test_state_transition_transient_failure_tolerance() {
    let transition = ProfileStateTransition::default(); // max_transient_failures_without_read = 2
                                                        // 1 transient failure followed by successful read matching target
    let reads = vec![Err("APDU channel busy".to_string()), Ok(Some(false))];

    let result = transition
        .evaluate_convergence(false, 1, &reads)
        .expect("converged after transient error");

    assert_eq!(
        result,
        ProfileStateTransitionResult {
            command_attempts: 1,
            verification_attempts: 2,
            actual_enabled: false,
        }
    );
}

#[test]
fn test_state_transition_transient_failure_threshold_exceeded() {
    let transition = ProfileStateTransition::default(); // max_transient_failures_without_read = 2
    let reads = vec![
        Err("Channel disconnected".to_string()),
        Err("Modem not responding".to_string()),
    ];

    let err = transition
        .evaluate_convergence(true, 1, &reads)
        .unwrap_err();

    match err {
        StateTransitionError::NonConvergence {
            target_enabled,
            command_attempts,
            verification_attempts,
            actual_enabled,
            last_error,
        } => {
            assert!(target_enabled);
            assert_eq!(command_attempts, 1);
            assert_eq!(verification_attempts, 2);
            assert_eq!(actual_enabled, None);
            assert_eq!(last_error, Some("Modem not responding".to_string()));
        }
        other => panic!("Unexpected error: {other:?}"),
    }
}

#[test]
fn test_state_transition_exhausted_reads_non_convergence() {
    let transition = ProfileStateTransition::new(2, 4, 3);
    let reads = vec![Ok(Some(false)), Ok(Some(false)), Ok(Some(false))];

    let err = transition
        .evaluate_convergence(true, 2, &reads)
        .unwrap_err();

    match err {
        StateTransitionError::NonConvergence {
            target_enabled,
            command_attempts,
            verification_attempts,
            actual_enabled,
            last_error,
        } => {
            assert!(target_enabled);
            assert_eq!(command_attempts, 2);
            assert_eq!(verification_attempts, 3);
            assert_eq!(actual_enabled, Some(false));
            assert_eq!(last_error, None);
        }
        other => panic!("Unexpected error: {other:?}"),
    }
}
