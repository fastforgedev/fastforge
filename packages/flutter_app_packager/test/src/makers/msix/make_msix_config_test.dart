import 'package:flutter_app_packager/src/makers/msix/make_msix_config.dart';
import 'package:test/test.dart';
import 'package:yaml/yaml.dart';

void main() {
  group('MakeMsixConfig', () {
    group('fromJson', () {
      test('accepts natural YAML scalars for string-typed options', () {
        final yaml = loadYaml('''
trim_logo: false
store: true
debug: false
sign_msix: false
install_certificate: true
enable_at_startup: true
add_execution_alias: true
build_windows: false
msix_version: 1
''') as YamlMap;
        final config =
            MakeMsixConfig.fromJson(Map<String, dynamic>.from(yaml));

        expect(config.trim_logo, equals('false'));
        expect(config.store, equals('true'));
        expect(config.debug, equals('false'));
        expect(config.sign_msix, equals('false'));
        expect(config.install_certificate, equals('true'));
        expect(config.enable_at_startup, equals('true'));
        expect(config.add_execution_alias, equals('true'));
        expect(config.build_windows, equals('false'));
        expect(config.msix_version, equals('1'));
      });

      test('keeps string values and leaves missing options null', () {
        final config = MakeMsixConfig.fromJson({
          'display_name': 'Demo',
          'trim_logo': 'false',
        });

        expect(config.display_name, equals('Demo'));
        expect(config.trim_logo, equals('false'));
        expect(config.store, isNull);
        expect(config.toJson().containsKey('store'), isFalse);
      });
    });
  });
}
