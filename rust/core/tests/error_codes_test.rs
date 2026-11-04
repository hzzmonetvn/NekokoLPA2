use nlpa_core::error_codes::{AppErrorCode, AppException};

#[test]
fn app_exception_display_includes_optional_fields() {
    let exception = AppException::new(
        AppErrorCode::ErrorUnknown,
        Some("failed".into()),
        Some("cause".into()),
    );
    assert_eq!(
        exception.to_string(),
        "AppException: ERROR_UNKNOWN (failed)\nOriginal Error: cause"
    );
    assert_eq!(
        AppException::new(AppErrorCode::ErrorUnknown, None, None).to_string(),
        "AppException: ERROR_UNKNOWN"
    );
}

#[test]
fn app_error_codes_cover_non_bluetooth_dart_variants() {
    let codes = [
        AppErrorCode::ErrorOmapiNotSupported,
        AppErrorCode::ErrorOmapiPermissionDenied,
        AppErrorCode::ErrorOmapiSecurityexception,
        AppErrorCode::ErrorOmapiReaderNotAvailable,
        AppErrorCode::ErrorOmapiChannelOpenFailed,
        AppErrorCode::ErrorOmapiTransmitFailed,
        AppErrorCode::ErrorOmapiSessionCorrupted,
        AppErrorCode::ErrorRemoteConnectionFailed,
        AppErrorCode::ErrorRemoteTimeout,
        AppErrorCode::ErrorRemoteAuthFailed,
        AppErrorCode::ErrorRemoteWrongPassword,
        AppErrorCode::ErrorCardNotPresent,
        AppErrorCode::ErrorCardUnsupported,
        AppErrorCode::ErrorConditionOfUseNotSatisfied,
        AppErrorCode::ErrorApplicationNotFound,
        AppErrorCode::ErrorEstkMaxNotFound,
        AppErrorCode::ErrorTransactionFailed,
        AppErrorCode::ErrorChannelOpenFailed,
        AppErrorCode::ErrorNblesimSimMode,
        AppErrorCode::ErrorUnknown,
    ];
    assert_eq!(codes.len(), 20);
    assert_eq!(codes[0].to_string(), "ERROR_OMAPI_NOT_SUPPORTED");
    assert_eq!(codes[19].to_string(), "ERROR_UNKNOWN");
}
