import 'package:flutter_app_publisher/src/api/app_package_publisher.dart';
import 'package:flutter_app_publisher/src/publishers/pgyer/publish_pgyer_config.dart';
import 'package:test/test.dart';

void main() {
  group('PublishPgyerConfig', () {
    group('constructor', () {
      test('creates instance with all parameters', () {
        final config = PublishPgyerConfig(
          apiKey: 'test_api_key',
          oversea: 1,
          buildInstallType: 2,
          buildPassword: 'test_password',
          buildDescription: 'Test app description',
          buildUpdateDescription: 'Test update description',
          buildInstallDate: 1,
          buildInstallStartDate: '2024-01-01',
          buildInstallEndDate: '2024-12-31',
          buildChannelShortcut: 'test_channel',
        );

        expect(config.apiKey, equals('test_api_key'));
        expect(config.oversea, equals(1));
        expect(config.buildInstallType, equals(2));
        expect(config.buildPassword, equals('test_password'));
        expect(config.buildDescription, equals('Test app description'));
        expect(
            config.buildUpdateDescription, equals('Test update description'));
        expect(config.buildInstallDate, equals(1));
        expect(config.buildInstallStartDate, equals('2024-01-01'));
        expect(config.buildInstallEndDate, equals('2024-12-31'));
        expect(config.buildChannelShortcut, equals('test_channel'));
      });

      test('creates instance with only required parameters', () {
        final config = PublishPgyerConfig(
          apiKey: 'test_api_key',
        );

        expect(config.apiKey, equals('test_api_key'));
        expect(config.oversea, isNull);
        expect(config.buildInstallType, isNull);
        expect(config.buildPassword, isNull);
        expect(config.buildDescription, isNull);
        expect(config.buildUpdateDescription, isNull);
        expect(config.buildInstallDate, isNull);
        expect(config.buildInstallStartDate, isNull);
        expect(config.buildInstallEndDate, isNull);
        expect(config.buildChannelShortcut, isNull);
      });
    });

    group('parse', () {
      test('parses with valid environment and all arguments', () {
        final environment = {'PGYER_API_KEY': 'test_api_key'};
        final publishArguments = {
          'oversea': 1,
          'install-type': 2,
          'password': 'test_password',
          'description': 'Test app description',
          'update-description': 'Test update description',
          'install-date': 1,
          'install-start-date': '2024-01-01',
          'install-end-date': '2024-12-31',
          'channel-shortcut': 'test_channel',
        };

        print('🔧 Environment: $environment');
        print('📝 Publish Arguments: $publishArguments');

        final config = PublishPgyerConfig.parse(environment, publishArguments);

        print('✅ Parsed Config:');
        print('   API Key: ${config.apiKey}');
        print('   Oversea: ${config.oversea}');
        print('   Install Type: ${config.buildInstallType}');
        print('   Password: ${config.buildPassword}');
        print('   Description: ${config.buildDescription}');
        print('   Update Description: ${config.buildUpdateDescription}');
        print('   Install Date: ${config.buildInstallDate}');
        print('   Install Start Date: ${config.buildInstallStartDate}');
        print('   Install End Date: ${config.buildInstallEndDate}');
        print('   Channel Shortcut: ${config.buildChannelShortcut}');

        expect(config.apiKey, equals('test_api_key'));
        expect(config.oversea, equals(1));
        expect(config.buildInstallType, equals(2));
        expect(config.buildPassword, equals('test_password'));
        expect(config.buildDescription, equals('Test app description'));
        expect(
            config.buildUpdateDescription, equals('Test update description'));
        expect(config.buildInstallDate, equals(1));
        expect(config.buildInstallStartDate, equals('2024-01-01'));
        expect(config.buildInstallEndDate, equals('2024-12-31'));
        expect(config.buildChannelShortcut, equals('test_channel'));
      });

      test('parses with only required parameters', () {
        final environment = {'PGYER_API_KEY': 'test_api_key'};
        final publishArguments = <String, dynamic>{};

        final config = PublishPgyerConfig.parse(environment, publishArguments);

        expect(config.apiKey, equals('test_api_key'));
        expect(config.oversea, isNull);
        expect(config.buildInstallType, isNull);
        expect(config.buildPassword, isNull);
        expect(config.buildDescription, isNull);
        expect(config.buildUpdateDescription, isNull);
        expect(config.buildInstallDate, isNull);
        expect(config.buildInstallStartDate, isNull);
        expect(config.buildInstallEndDate, isNull);
        expect(config.buildChannelShortcut, isNull);
      });

      test('parses with partial arguments', () {
        final environment = {'PGYER_API_KEY': 'test_api_key'};
        final publishArguments = {
          'description': 'Test app description',
          'install-type': 2,
          'password': 'test_password',
        };

        final config = PublishPgyerConfig.parse(environment, publishArguments);

        expect(config.apiKey, equals('test_api_key'));
        expect(config.buildDescription, equals('Test app description'));
        expect(config.buildInstallType, equals(2));
        expect(config.buildPassword, equals('test_password'));
        expect(config.oversea, isNull);
        expect(config.buildUpdateDescription, isNull);
        expect(config.buildInstallDate, isNull);
        expect(config.buildInstallStartDate, isNull);
        expect(config.buildInstallEndDate, isNull);
        expect(config.buildChannelShortcut, isNull);
      });

      test('handles string values for numeric parameters', () {
        final environment = {'PGYER_API_KEY': 'test_api_key'};
        final publishArguments = {
          'oversea': '1',
          'install-type': '2',
          'install-date': '1',
        };

        print('🔄 String to Int Conversion Test:');
        print('   Input Arguments: $publishArguments');
        print('   Expected: oversea=1, install-type=2, install-date=1');

        final config = PublishPgyerConfig.parse(environment, publishArguments);

        print(
            '   Result: oversea=${config.oversea}, install-type=${config.buildInstallType}, install-date=${config.buildInstallDate}');

        expect(config.oversea, equals(1));
        expect(config.buildInstallType, equals(2));
        expect(config.buildInstallDate, equals(1));
      });

      test('throws PublishError when PGYER_API_KEY is missing', () {
        final environment = <String, String>{};
        final publishArguments = <String, dynamic>{};

        print('❌ Error Test - Missing API Key:');
        print('   Environment: $environment');
        print('   Expected: PublishError with missing API key message');

        expect(
          () => PublishPgyerConfig.parse(environment, publishArguments),
          throwsA(
            isA<PublishError>().having(
              (e) => e.message,
              'message',
              contains('Missing `PGYER_API_KEY` environment variable'),
            ),
          ),
        );
      });

      test('throws PublishError when PGYER_API_KEY is empty', () {
        final environment = {'PGYER_API_KEY': ''};
        final publishArguments = <String, dynamic>{};

        expect(
          () => PublishPgyerConfig.parse(environment, publishArguments),
          throwsA(
            isA<PublishError>().having(
              (e) => e.message,
              'message',
              contains('Missing `PGYER_API_KEY` environment variable'),
            ),
          ),
        );
      });

      test('handles null publishArguments', () {
        final environment = {'PGYER_API_KEY': 'test_api_key'};

        final config = PublishPgyerConfig.parse(environment, null);

        expect(config.apiKey, equals('test_api_key'));
        expect(config.oversea, isNull);
        expect(config.buildInstallType, isNull);
        expect(config.buildPassword, isNull);
        expect(config.buildDescription, isNull);
        expect(config.buildUpdateDescription, isNull);
        expect(config.buildInstallDate, isNull);
        expect(config.buildInstallStartDate, isNull);
        expect(config.buildInstallEndDate, isNull);
        expect(config.buildChannelShortcut, isNull);
      });

      // Note: Testing null environment is difficult because it falls back to
      // Platform.environment, which depends on actual system environment variables.
      // This test is skipped to avoid flaky behavior.
    });
  });
}
