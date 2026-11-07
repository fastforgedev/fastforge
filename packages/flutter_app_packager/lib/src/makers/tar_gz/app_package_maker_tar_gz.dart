import 'dart:io';

import 'package:archive/archive_io.dart';
import 'package:flutter_app_packager/src/api/app_package_maker.dart';
import 'package:path/path.dart' as path;

class AppPackageMakerTarGz extends AppPackageMaker {
  AppPackageMakerTarGz(this.platform);

  @override
  final String platform;
  @override
  String get name => 'tar.gz';
  @override
  String get packageFormat => 'tar.gz';

  @override
  Future<MakeResult> make(MakeConfig config) async {
    final source = config.buildOutputDirectory;
    if (!source.existsSync()) {
      throw MakeError('Directory not found: ${source.path}');
    }
    final roots = source.listSync(followLinks: false)
      ..sort((a, b) => a.path.compareTo(b.path));
    if (platform == 'macos') {
      roots.removeWhere((entry) => !entry.path.endsWith('.app'));
      if (roots.isEmpty) {
        throw MakeError('.app bundle not found in build output');
      }
    }
    final output = config.outputFile;
    final temp = Directory.systemTemp.createTempSync('fastforge_tar_');
    try {
      final tarFile = File(path.join(temp.path, 'archive.tar'));
      final stream = OutputFileStream(tarFile.path);
      final encoder = TarEncoder()..start(stream);
      try {
        for (final entry in roots) {
          await _addEntry(encoder, entry, source, output);
        }
        encoder.finish();
      } finally {
        await stream.close();
      }
      await tarFile.openRead().transform(gzip.encoder).pipe(output.openWrite());
    } finally {
      temp.deleteSync(recursive: true);
    }
    return resultResolver.resolve(config);
  }

  Future<void> _addEntry(
    TarEncoder encoder,
    FileSystemEntity entry,
    Directory source,
    File output,
  ) async {
    if (path.equals(entry.absolute.path, output.absolute.path) ||
        (output.existsSync() &&
            FileSystemEntity.identicalSync(entry.path, output.path))) {
      return;
    }
    final name = path.relative(entry.path, from: source.path);
    if (entry is Link) {
      encoder.add(
        ArchiveFile(name, 0, <int>[])
          ..isSymbolicLink = true
          ..nameOfLinkedFile = entry.targetSync(),
      );
    } else if (entry is Directory) {
      encoder.add(
        ArchiveFile('$name/', 0, <int>[])
          ..isFile = false
          ..mode = entry.statSync().mode,
      );
      final children = entry.listSync(followLinks: false)
        ..sort((a, b) => a.path.compareTo(b.path));
      for (final child in children) {
        await _addEntry(encoder, child, source, output);
      }
    } else if (entry is File) {
      final input = InputFileStream(entry.path);
      try {
        encoder.add(
          ArchiveFile.stream(name, entry.lengthSync(), input)
            ..mode = entry.statSync().mode
            ..lastModTime =
                entry.lastModifiedSync().millisecondsSinceEpoch ~/ 1000,
        );
      } finally {
        await input.close();
      }
    }
  }
}
