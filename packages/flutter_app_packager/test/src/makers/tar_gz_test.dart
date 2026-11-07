import 'dart:convert';
import 'dart:io';

import 'package:archive/archive_io.dart';
import 'package:flutter_app_packager/src/api/make_error.dart';
import 'package:flutter_app_packager/src/flutter_app_packager.dart';
import 'package:flutter_app_packager/src/makers/tar_gz/app_package_maker_tar_gz.dart';
import 'package:test/test.dart';

void main() {
  late Directory current;
  late Directory temp;

  setUp(() {
    current = Directory.current;
    temp = Directory.systemTemp.createTempSync('fastforge_tar_test_');
    Directory.current = temp;
    File('pubspec.yaml').writeAsStringSync('name: demo\nversion: 1.0.0+1\n');
    Directory('bundle/Demo.app/Contents/empty').createSync(recursive: true);
    File('bundle/Demo.app/Contents/demo').writeAsStringSync('binary');
    File('bundle/.hidden').writeAsStringSync('hidden');
  });

  tearDown(() {
    Directory.current = current;
    temp.deleteSync(recursive: true);
  });

  for (final platform in ['linux', 'macos', 'windows', 'web']) {
    test('$platform tar.gz target creates and overwrites a usable archive',
        () async {
      final packager = FlutterAppPackager();
      for (var run = 0; run < 2; run++) {
        final result = await packager.package(
          platform,
          'tar.gz',
          {'build_mode': 'release'},
          Directory('bundle/dist/'),
          buildOutputDirectory: Directory('bundle'),
          buildOutputFiles: [],
        );
        final output = File(result.artifacts.single.path);
        expect(output.path, endsWith('demo-1.0.0+1-$platform.tar.gz'));
        final archive =
            TarDecoder().decodeBytes(gzip.decode(output.readAsBytesSync()));
        expect(
          archive.findFile('Demo.app/Contents/demo')?.content,
          utf8.encode('binary'),
        );
        expect(archive.findFile('Demo.app/Contents/empty/'), isNotNull);
        expect(archive.findFile('.hidden') != null, platform != 'macos');
        expect(
          archive.files.any((entry) => entry.name.endsWith('.tar.gz')),
          isFalse,
        );
      }
    });
  }

  test(
    'tar.gz preserves executable permissions and symbolic links',
    () async {
      Process.runSync('chmod', ['755', 'bundle/Demo.app/Contents/demo']);
      Link('bundle/link').createSync('Demo.app/Contents/demo');
      Link('bundle/cycle').createSync('.');
      Link('bundle/dangling').createSync('missing');
      final result = await FlutterAppPackager().package(
        'linux',
        'tar.gz',
        {'build_mode': 'release'},
        Directory('dist/'),
        buildOutputDirectory: Directory('bundle'),
        buildOutputFiles: [],
      );
      final archive = TarDecoder().decodeBytes(
        gzip.decode(File(result.artifacts.single.path).readAsBytesSync()),
      );
      expect(archive.findFile('Demo.app/Contents/demo')!.mode & 0x1ff, 0x1ed);
      for (final name in ['link', 'cycle', 'dangling']) {
        expect(archive.findFile(name)!.isSymbolicLink, isTrue);
      }
      expect(
        archive.findFile('link')!.nameOfLinkedFile,
        'Demo.app/Contents/demo',
      );
      expect(archive.findFile('cycle')!.nameOfLinkedFile, '.');
      expect(archive.findFile('dangling')!.nameOfLinkedFile, 'missing');
    },
    skip: Platform.isWindows ? 'Unix permissions and symlinks' : false,
  );

  test('tar.gz rejects a missing macOS app', () async {
    final maker = AppPackageMakerTarGz('macos');
    Directory('bundle/Demo.app').deleteSync(recursive: true);
    final config = maker.configLoader.load(
      {'build_mode': 'release'},
      Directory('dist/'),
      buildOutputDirectory: Directory('bundle'),
      buildOutputFiles: [],
    );
    await expectLater(maker.make(config), throwsA(isA<MakeError>()));
  });

  test('tar.gz preserves Unicode and long paths in standard PAX headers',
      () async {
    final longName = List.filled(40, '中文').join();
    final names = ['资料/中文.txt', '资料/$longName.txt'];
    Directory('bundle/资料').createSync();
    for (final name in names) {
      File('bundle/$name').writeAsStringSync('Unicode payload');
    }
    if (!Platform.isWindows) {
      Link('bundle/中文链接').createSync(names.last);
    }
    final result = await FlutterAppPackager().package(
      'linux',
      'tar.gz',
      {'build_mode': 'release'},
      Directory('dist/'),
      buildOutputDirectory: Directory('bundle'),
      buildOutputFiles: [],
    );
    final bytes = gzip.decode(
      File(result.artifacts.single.path).readAsBytesSync(),
    );
    final archive = TarDecoder().decodeBytes(bytes);
    for (final name in names) {
      expect(archive.findFile(name)?.content, utf8.encode('Unicode payload'));
    }
    if (!Platform.isWindows) {
      final link = archive.findFile('中文链接')!;
      expect(link.isSymbolicLink, isTrue);
      expect(link.nameOfLinkedFile, names.last);
    }
  });
}
