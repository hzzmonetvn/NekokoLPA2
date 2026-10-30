import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:nlpa2/settings/app_settings.dart';
import 'package:nlpa2/theme/app_theme.dart';

void main() {
  test('AMOLED theme uses black system bars and accessible dark colors', () {
    final theme = AppTheme.theme(true, AppThemeType.amoled);

    expect(theme.scaffoldBackgroundColor, Colors.black);
    expect(theme.appBarTheme.backgroundColor, Colors.black);
    expect(theme.appBarTheme.systemOverlayStyle?.systemNavigationBarColor,
        Colors.black);
    expect(theme.appBarTheme.systemOverlayStyle?.statusBarIconBrightness,
        Brightness.light);
    expect(theme.colorScheme.surface, const Color(0xFF101010));
    expect(theme.colorScheme.onSurface, const Color(0xFFF5F5F5));
    expect(AppTheme.theme(true, AppThemeType.stock).scaffoldBackgroundColor,
        isNot(Colors.black));
  });
}
