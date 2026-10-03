import 'dart:io';

import 'package:flutter_app_packager/src/makers/appimage/make_appimage_config.dart';
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
      expect(files['DESKTOP'], contains('Icon=test_app'));
      expect(files['DESKTOP'], contains('Exec=test_app %U'));
    });

    test('rpm keeps package, project, and binary names independent', () {
      File('${tempDir.path}/linux/CMakeLists.txt')
          .writeAsStringSync('set(BINARY_NAME "test_binary")\n');
      final config = MakeRPMConfig(
        displayName: 'Test App',
        packageName: 'test-package',
      )..pubspec = _pubspec();

      final files = config.toFilesString();

      expect(files['SPEC'], contains('Name: test-package'));
      expect(
        files['SPEC'],
        contains('cp -r test_app/* %{buildroot}%{_datadir}/%{name}'),
      );
      expect(
        files['SPEC'],
        contains(
          'ln -s %{_datadir}/%{name}/test_binary '
          '%{buildroot}%{_bindir}/%{name}',
        ),
      );
      expect(files['DESKTOP'], contains('Icon=test-package'));
      expect(files['DESKTOP'], contains('Exec=test-package %U'));
    });

    test('rpm places lifecycle scripts outside the install section', () {
      final config = MakeRPMConfig.fromJson({
        'display_name': 'Test App',
        'postinstall_scripts': ['echo postinstall'],
        'postuninstall_scripts': ['echo postuninstall'],
      })..pubspec = _pubspec();

      final spec = config.toFilesString()['SPEC']!;
      final installSection = spec.substring(
        spec.indexOf('%install'),
        spec.indexOf('\n%post\n'),
      );

      expect(installSection, isNot(contains('update-mime-database')));
      expect(
        spec,
        contains(
          '%post\n'
          'update-mime-database %{_datadir}/mime &> /dev/null || :\n'
          'echo postinstall',
        ),
      );
      expect(
        spec,
        contains(
          '%postun\n'
          'update-mime-database %{_datadir}/mime &> /dev/null || :\n'
          'echo postuninstall',
        ),
      );
    });

    test('rpm retains the legacy postun constructor parameter', () {
      final config = MakeRPMConfig(
        displayName: 'Test App',
        postun: 'echo legacy-postun',
      )..pubspec = _pubspec();

      expect(
        config.toFilesString()['SPEC'],
        contains('echo legacy-postun'),
      );
    });

    test('appimage launches the CMake binary, not the pubspec name', () {
      File('${tempDir.path}/linux/CMakeLists.txt')
          .writeAsStringSync('set(BINARY_NAME "test_binary")\n');
      final config = MakeAppImageConfig.fromJson({
        'display_name': 'Test App',
        'icon': 'icon.png',
        'actions': [
          {
            'label': 'NewWindow',
            'name': 'New Window',
            'arguments': ['--new-window'],
          },
        ],
      })..pubspec = _pubspec();

      expect(config.appRunContent, contains('exec ./test_binary\n'));
      expect(
        config.desktopFileContent,
        contains('Exec=LD_LIBRARY_PATH=usr/lib test_binary %u'),
      );
      expect(
        config.desktopFileContent,
        contains('Exec=LD_LIBRARY_PATH=usr/lib test_binary --new-window %u'),
      );
      expect(config.desktopFileContent, contains('Icon=test_app'));
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
