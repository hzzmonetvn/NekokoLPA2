import 'dart:io';
import 'dart:typed_data';

import 'package:path/path.dart' as path;
import 'package:path_provider/path_provider.dart';

class BackgroundImageStore {
  BackgroundImageStore({Future<Directory> Function()? supportDirectory})
    : _supportDirectory = supportDirectory ?? getApplicationSupportDirectory;

  static const _directoryName = 'background';
  static const _fileName = 'app_background';
  final Future<Directory> Function() _supportDirectory;

  Future<String> save(Uint8List bytes, {String? extension}) async {
    final directory = await _supportDirectory();
    final imageDirectory = Directory(path.join(directory.path, _directoryName));
    await imageDirectory.create(recursive: true);

    final normalizedExtension = _normalizeExtension(extension);
    final uniqueSuffix =
        '${DateTime.now().microsecondsSinceEpoch}-${identityHashCode(Object())}';
    final fileName = '$_fileName-$uniqueSuffix$normalizedExtension';
    final destination = File(path.join(imageDirectory.path, fileName));
    final temporary = File('${destination.path}.tmp');
    await temporary.writeAsBytes(bytes, flush: true);
    return (await temporary.rename(destination.path)).path;
  }

  Future<void> delete(String imagePath) async {
    final directory = await _supportDirectory();
    final ownedDirectory = path.join(directory.path, _directoryName);
    if (!path.isWithin(ownedDirectory, imagePath)) return;

    final file = File(imagePath);
    if (await file.exists()) await file.delete();
  }

  String _normalizeExtension(String? extension) {
    final value = extension?.toLowerCase().replaceFirst(RegExp(r'^\.'), '');
    if (value == null || !RegExp(r'^[a-z0-9]{1,8}$').hasMatch(value)) {
      return '.img';
    }
    return '.$value';
  }
}
