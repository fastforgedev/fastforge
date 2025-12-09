import 'package:flutter_app_publisher/src/api/app_package_publisher.dart';
import 'package:flutter_app_publisher/src/publishers/github/publish_github_config.dart';
import 'package:test/test.dart';

void main() {
  group('PublishGithubConfig', () {
    group('constructor', () {
      test('creates instance with all parameters', () {
        final config = PublishGithubConfig(
          appVersion: '1.0.0+1',
          token: 'test_token',
          repository: 'owner/repo',
          releaseTitle: 'v1.0.0',
        );

        expect(config.token, equals('test_token'));
        expect(config.repository, equals('owner/repo'));
        expect(config.releaseTitle, equals('v1.0.0'));
      });

      test('creates instance with optional releaseTitle as null', () {
        final config = PublishGithubConfig(
          appVersion: '1.0.0+1',
          token: 'test_token',
          repository: 'owner/repo',
        );

        expect(config.releaseTitle, isNull);
      });
    });

    group('parse', () {
      test('parses with valid environment and arguments', () {
        final environment = {'GITHUB_TOKEN': 'test_token'};
        final publishArguments = {
          'app-version': '1.0.0+1',
          'repo': 'owner/repo',
          'release-title': 'Release v{appVersion}',
        };

        final config = PublishGithubConfig.parse(environment, publishArguments);

        expect(config.token, equals('test_token'));
        expect(config.repository, equals('owner/repo'));
        expect(config.releaseTitle, equals('Release v1.0.0'));
      });

      test('uses default releaseTitle when not provided', () {
        final environment = {'GITHUB_TOKEN': 'test_token'};
        final publishArguments = {
          'app-version': '1.0.0+1',
          'repo': 'owner/repo',
        };

        final config = PublishGithubConfig.parse(environment, publishArguments);

        expect(config.releaseTitle, equals('v1.0.0+1'));
      });

      test('replaces placeholders in releaseTitle', () {
        final environment = {'GITHUB_TOKEN': 'test_token'};
        final publishArguments = {
          'app-version': '2.1.0+15',
          'repo': 'owner/repo',
          'release-title': 'v{appBuildName} build {appBuildNumber}',
        };

        final config = PublishGithubConfig.parse(environment, publishArguments);

        expect(config.releaseTitle, equals('v2.1.0 build 15'));
      });

      test('falls back to GITHUB_REPOSITORY when repo not provided', () {
        final environment = {
          'GITHUB_TOKEN': 'test_token',
          'GITHUB_REPOSITORY': 'env_owner/env_repo',
        };
        final publishArguments = {'app-version': '1.0.0+1'};

        final config = PublishGithubConfig.parse(environment, publishArguments);

        expect(config.repository, equals('env_owner/env_repo'));
      });

      test('prioritizes repo argument over GITHUB_REPOSITORY', () {
        final environment = {
          'GITHUB_TOKEN': 'test_token',
          'GITHUB_REPOSITORY': 'env_owner/env_repo',
        };
        final publishArguments = {
          'app-version': '1.0.0+1',
          'repo': 'arg_owner/arg_repo',
        };

        final config = PublishGithubConfig.parse(environment, publishArguments);

        expect(config.repository, equals('arg_owner/arg_repo'));
      });

      test('throws PublishError when GITHUB_TOKEN is missing', () {
        final environment = <String, String>{};
        final publishArguments = {'repo': 'owner/repo'};

        expect(
          () => PublishGithubConfig.parse(environment, publishArguments),
          throwsA(
            isA<PublishError>().having(
              (e) => e.message,
              'message',
              contains('Missing `GITHUB_TOKEN` environment variable'),
            ),
          ),
        );
      });

      test('throws PublishError when repository is missing', () {
        final environment = {'GITHUB_TOKEN': 'test_token'};
        final publishArguments = <String, dynamic>{};

        expect(
          () => PublishGithubConfig.parse(environment, publishArguments),
          throwsA(
            isA<PublishError>().having(
              (e) => e.message,
              'message',
              contains('GitHub repository is required'),
            ),
          ),
        );
      });

      test('throws PublishError for invalid repository format', () {
        final environment = {'GITHUB_TOKEN': 'test_token'};
        final publishArguments = {'repo': 'invalid_format'};

        expect(
          () => PublishGithubConfig.parse(environment, publishArguments),
          throwsA(
            isA<PublishError>().having(
              (e) => e.message,
              'message',
              contains('Invalid GitHub repository format'),
            ),
          ),
        );
      });
    });
  });
}
