import 'package:flutter_app_packager/src/makers/pkg/make_pkg_config.dart';
import 'package:test/test.dart';

void main() {
  group('MakePkgConfig', () {
    group('fromJson', () {
      test('parses all fields', () {
        final config = MakePkgConfig.fromJson({
          'install-path': '/Applications',
          'sign-identity': 'Developer ID Installer: Test (TEAM123)',
          'scripts': '/path/to/scripts',
        });

        expect(config.installPath, equals('/Applications'));
        expect(
          config.signIdentity,
          equals('Developer ID Installer: Test (TEAM123)'),
        );
        expect(config.scriptsPath, equals('/path/to/scripts'));
      });

      test('parses minimal config with only install-path', () {
        final config = MakePkgConfig.fromJson({
          'install-path': '/Applications',
        });

        expect(config.installPath, equals('/Applications'));
        expect(config.signIdentity, isNull);
        expect(config.scriptsPath, isNull);
      });

      test('parses empty json', () {
        final config = MakePkgConfig.fromJson({});

        expect(config.installPath, isNull);
        expect(config.signIdentity, isNull);
        expect(config.scriptsPath, isNull);
      });
    });

    group('toJson', () {
      test('outputs all fields when set', () {
        final config = MakePkgConfig(
          installPath: '/Applications',
          signIdentity: 'Developer ID Installer: Test (TEAM123)',
          scriptsPath: '/path/to/scripts',
        );

        final json = config.toJson();
        expect(json['install-path'], equals('/Applications'));
        expect(
          json['sign-identity'],
          equals('Developer ID Installer: Test (TEAM123)'),
        );
        expect(json['scripts'], equals('/path/to/scripts'));
      });

      test('omits null fields', () {
        final config = MakePkgConfig(
          installPath: '/Applications',
        );

        final json = config.toJson();
        expect(json['install-path'], equals('/Applications'));
        expect(json.containsKey('sign-identity'), isFalse);
        expect(json.containsKey('scripts'), isFalse);
      });

      test('round-trip preserves all fields', () {
        final original = {
          'install-path': '/Applications',
          'sign-identity': 'Developer ID Installer: Test (TEAM123)',
          'scripts': '/path/to/scripts',
        };

        final config = MakePkgConfig.fromJson(original);
        final json = config.toJson();

        expect(json, equals(original));
      });
    });
  });
}
