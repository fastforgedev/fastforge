import 'dart:io';

import 'package:flutter_app_packager/src/api/app_package_maker.dart';
import 'package:pub_semver/pub_semver.dart';
import 'package:pubspec_parse/pubspec_parse.dart';
import 'package:test/test.dart';

void main() {
  group('MakeConfig', () {
    test('#1', () {
      final makeConfig = MakeConfig()
        ..buildMode = 'release'
        ..buildOutputDirectory = Directory('build')
        ..buildOutputFiles = []
        ..platform = 'android'
        ..packageFormat = 'apk'
        ..outputDirectory = Directory('dist/')
        ..pubspec = Pubspec(
          'test_app',
          version: Version.parse('1.0.0'),
        );
      expect(
        makeConfig.outputArtifactPath,
        'dist/1.0.0/test_app-1.0.0-android.apk',
      );
    });
    test('#2', () {
      final makeConfig = MakeConfig()
        ..buildMode = 'release'
        ..buildOutputDirectory = Directory('build')
        ..buildOutputFiles = []
        ..platform = 'android'
        ..packageFormat = 'apk'
        ..outputDirectory = Directory('dist/')
        ..pubspec = Pubspec(
          'test_app',
          version: Version.parse('1.0.0+1'),
        );
      expect(
        makeConfig.outputArtifactPath,
        'dist/1.0.0+1/test_app-1.0.0+1-android.apk',
      );
    });
  });

  group('loadMakeConfigYaml', () {
    late Directory tempDir;

    setUp(() {
      tempDir = Directory.systemTemp.createTempSync('fastforge_test_');
    });

    tearDown(() {
      if (tempDir.existsSync()) {
        tempDir.deleteSync(recursive: true);
      }
    });

    test('should merge default config with specific config', () {
      final platformDir = Directory('${tempDir.path}/linux/packaging');
      final formatDir = Directory('${platformDir.path}/deb');
      formatDir.createSync(recursive: true);

      final defaultConfig = File('${platformDir.path}/make_config.yaml');
      defaultConfig.writeAsStringSync('a: 1\nb:\n  c: 2\n  d: 3\n');

      final specificConfig = File('${formatDir.path}/make_config.yaml');
      specificConfig.writeAsStringSync('b:\n  d: 4\n  e: 5\nf: 6\n');

      final result = loadMakeConfigYaml(specificConfig.path);

      expect(result['a'], 1);
      expect(result['b']['c'], 2);
      expect(result['b']['d'], 4);
      expect(result['b']['e'], 5);
      expect(result['f'], 6);
    });

    test('should load specific config if default is missing', () {
      final platformDir = Directory('${tempDir.path}/linux/packaging');
      final formatDir = Directory('${platformDir.path}/deb');
      formatDir.createSync(recursive: true);

      final specificConfig = File('${formatDir.path}/make_config.yaml');
      specificConfig.writeAsStringSync('a: 1\n');

      final result = loadMakeConfigYaml(specificConfig.path);

      expect(result['a'], 1);
    });

    test('should load default config if specific is missing', () {
      final platformDir = Directory('${tempDir.path}/linux/packaging');
      final formatDir = Directory('${platformDir.path}/deb');
      formatDir.createSync(recursive: true);

      final defaultConfig = File('${platformDir.path}/make_config.yaml');
      defaultConfig.writeAsStringSync('a: 1\n');

      final specificConfig = File('${formatDir.path}/make_config.yaml');
      final result = loadMakeConfigYaml(specificConfig.path);

      expect(result['a'], 1);
    });

    test('should throw FileSystemException if neither config exists', () {
      final platformDir = Directory('${tempDir.path}/linux/packaging');
      final formatDir = Directory('${platformDir.path}/deb');
      formatDir.createSync(recursive: true);

      final specificConfig = File('${formatDir.path}/make_config.yaml');

      expect(
        () => loadMakeConfigYaml(specificConfig.path),
        throwsA(isA<FileSystemException>()),
      );
    });

    test('should throw FormatException if default config is invalid', () {
      final platformDir = Directory('${tempDir.path}/linux/packaging');
      final formatDir = Directory('${platformDir.path}/deb');
      formatDir.createSync(recursive: true);

      final defaultConfig = File('${platformDir.path}/make_config.yaml');
      defaultConfig.writeAsStringSync('- a\n- b\n');

      final specificConfig = File('${formatDir.path}/make_config.yaml');

      expect(
        () => loadMakeConfigYaml(specificConfig.path),
        throwsA(isA<FormatException>()),
      );
    });

    test('should throw FormatException if specific config is invalid', () {
      final platformDir = Directory('${tempDir.path}/linux/packaging');
      final formatDir = Directory('${platformDir.path}/deb');
      formatDir.createSync(recursive: true);

      final defaultConfig = File('${platformDir.path}/make_config.yaml');
      defaultConfig.writeAsStringSync('a: 1\n');

      final specificConfig = File('${formatDir.path}/make_config.yaml');
      specificConfig.writeAsStringSync('- a\n- b\n');

      expect(
        () => loadMakeConfigYaml(specificConfig.path),
        throwsA(isA<FormatException>()),
      );
    });
  });
}
