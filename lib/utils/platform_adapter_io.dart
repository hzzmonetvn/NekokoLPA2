import 'dart:io';

class PlatformX {
  static bool get isAndroid => Platform.isAndroid;
  static String get operatingSystem => Platform.operatingSystem;
  static bool get isWeb => false;
  static bool get isMobile => true;
  static bool get isDesktop => false;
  static bool get supportsBle => true;
}
