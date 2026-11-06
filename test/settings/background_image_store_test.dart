import 'dart:io';
import 'dart:typed_data';

import 'package:flutter_test/flutter_test.dart';
import 'package:nlpa2/settings/background_image_store.dart';
import 'package:path/path.dart' as path;

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  test(
    'saves bytes in app support storage and deletes only owned files',
    () async {
      final directory = await Directory.systemTemp.createTemp(
        'background-test',
      );
      final store = BackgroundImageStore(
        supportDirectory: () async => directory,
      );
      final bytes = Uint8List.fromList([1, 2, 3, 4]);

      final savedPath = await store.save(bytes, extension: 'png');
      expect(await File(savedPath).readAsBytes(), bytes);
      expect(savedPath, endsWith('.png'));

      final externalFile = File(
        path.join(directory.parent.path, 'unowned-background.png'),
      );
      await externalFile.writeAsBytes(bytes);
      await store.delete(externalFile.path);
      expect(await externalFile.exists(), isTrue);
      await store.delete(savedPath);
      expect(await File(savedPath).exists(), isFalse);

      await externalFile.delete();
      await directory.delete(recursive: true);
    },
  );
}
