class BuildCapabilities {
  BuildCapabilities._();

  static String variantForPlatform(String variant, {required bool isAndroid}) {
    if (!isAndroid && (variant == 'open' || variant == 'open-rooted')) {
      return 'std';
    }
    return variant;
  }

  static bool isEsim9Variant(String value) {
    final normalized = value.trim().toLowerCase();
    return normalized == 'esim9' || normalized == '9esim';
  }

  static bool isEsim9Build({
    required bool compileTimeFlag,
    required String environmentVariant,
    required String flavorVariant,
    required String packageName,
  }) {
    return compileTimeFlag ||
        isEsim9Variant(environmentVariant) ||
        isEsim9Variant(flavorVariant) ||
        isEsim9Variant(packageName) ||
        packageName == 'com.9esim.nlpa' ||
        packageName.endsWith('.flavor1');
  }

}
