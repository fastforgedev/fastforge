import 'dart:io';

import 'package:flutter_app_packager/src/makers/deb/make_deb_config.dart';
import 'package:flutter_app_packager/src/makers/pacman/make_pacman_config.dart';
import 'package:flutter_app_packager/src/makers/rpm/make_rpm_config.dart';
import 'package:pub_semver/pub_semver.dart';
import 'package:pubspec_parse/pubspec_parse.dart';
import 'package:test/test.dart';

void main() {
  group('Linux desktop entry files', () {
    late Directory originalCurrent;
    late Directory tempDir;

    setUp(() {
      originalCurrent = Directory.current;
      tempDir = Directory.systemTemp.createTempSync('fastforge_desktop_test_');
      Directory('${tempDir.path}/linux').createSync(recursive: true);
      File('${tempDir.path}/linux/CMakeLists.txt')
          .writeAsStringSync('set(BINARY_NAME "test_app")\n');
      Directory.current = tempDir;
    });

    tearDown(() {
      Directory.current = originalCurrent;
      tempDir.deleteSync(recursive: true);
    });

    test('deb keeps package version out of the desktop entry', () {
      final config = MakeDebConfig(
        displayName: 'Test App',
        packageName: 'test-app',
        installedSize: 42,
        maintainer: 'Maintainer <maintainer@example.com>',
      )..pubspec = _pubspec();

      final files = config.toFilesString();

      expect(files['CONTROL'], contains('Version: 1.2.3+4'));
      expect(files['DESKTOP'], isNot(contains('\nVersion=')));
    });

    test('rpm keeps package version out of the desktop entry', () {
      final config = MakeRPMConfig(
        displayName: 'Test App',
      )..pubspec = _pubspec();

      final files = config.toFilesString();

      expect(files['SPEC'], contains('Version: 1.2.3+4'));
      expect(files['DESKTOP'], isNot(contains('\nVersion=')));
    });

    test('pacman keeps package version out of the desktop entry', () {
      final config = MakePacmanConfig(
        displayName: 'Test App',
        packageName: 'test-app',
        maintainer: 'Maintainer <maintainer@example.com>',
      )..pubspec = _pubspec();

      final files = config.toFilesString();

      expect(files['PKGINFO'], contains('pkgver=1.2.3+4'));
      expect(files['DESKTOP'], isNot(contains('\nVersion=')));
    });
  });
}

Pubspec _pubspec() {
  return Pubspec(
    'test_app',
    version: Version.parse('1.2.3+4'),
    description: 'Test app',
    homepage: 'https://example.com',
  );
}
