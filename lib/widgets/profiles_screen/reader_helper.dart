import 'package:flutter/material.dart';
import '../../theme/app_theme.dart';

class ReaderHelper {
  static IconData getIconData(String? reader) {
    if (reader == null) return Icons.devices_other_rounded;

    if (reader.startsWith('omapi:')) {
      return Icons.sim_card_rounded;
    }

    if (reader.startsWith('tmapi:') || reader.contains('|telephony:')) {
      return Icons.smartphone_rounded;
    }

    if (reader.startsWith('remocard:')) {
      return Icons.cloud_rounded;
    }

    if (reader.startsWith('qrtr:') || reader.startsWith('hal:')) {
      return Icons.memory_rounded;
    }

    return Icons.usb_rounded;
  }

  static Color? getIconColor(
    String? reader,
    BuildContext context, {
    Color? overrideColor,
  }) {
    if (overrideColor != null) return overrideColor;

    final theme = Theme.of(context);
    final isDark = theme.brightness == Brightness.dark;
    final defaultSubtle = AppTheme.onSurfaceSubtle(context);

    if (reader == null) return defaultSubtle;

    if (reader.startsWith('omapi:')) {
      return AppTheme.simCardIconColor(context);
    }

    if (reader.startsWith('tmapi:') || reader.contains('|telephony:')) {
      return isDark ? Colors.teal[300] : Colors.teal[600];
    }

    if (reader.startsWith('remocard:')) {
      return isDark ? Colors.purple[300] : Colors.purple[600];
    }

    return defaultSubtle;
  }

  static bool isPersistent(String? readerId) {
    if (readerId == null) return false;
    return readerId.startsWith('remocard:');
  }
}
