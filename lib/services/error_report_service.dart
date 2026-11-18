/// Telemetry and Sentry error reporting are intentionally excised in this fork.
class ErrorReportService {
  static bool get isAvailable => false;

  static Future<String?> report(Object error, {required String kind}) async {
    return null;
  }
}
