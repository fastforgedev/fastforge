import 'dart:io';

import 'package:flutter_app_builder/src/build_config.dart';
import 'package:flutter_app_builder/src/build_error.dart';
import 'package:flutter_app_builder/src/build_result.dart';
import 'package:flutter_app_builder/src/commands/flutter.dart';
import 'package:pub_semver/pub_semver.dart';
import 'package:pubspec_parse/pubspec_parse.dart';

abstract class AppBuilder {
  String get platform => throw UnimplementedError();
  bool get isSupportedOnCurrentPlatform => throw UnimplementedError();
  BuildResultResolver get resultResolver;
  String get buildSubcommand => platform;

  String get appName => pubspec.name;
  Version get appVersion => pubspec.version!;
  String get appBuildName => appVersion.toString().split('+').first;
  String get appBuildNumber => appVersion.toString().split('+').last;

  Pubspec? _pubspec;

  Pubspec get pubspec {
    if (_pubspec == null) {
      final yamlString = File('pubspec.yaml').readAsStringSync();
      _pubspec = Pubspec.parse(yamlString);
    }
    return _pubspec!;
  }

  bool match(String platform, [String? target]) {
    return this.platform == platform;
  }

  Future<BuildResult> build({
    required Map<String, dynamic> arguments,
    Map<String, String>? environment,
  }) async {
    final time = Stopwatch()..start();

    BuildConfig config = BuildConfig(arguments: arguments);
    List<String> buildArguments = [];
    for (String key in config.arguments.keys) {
      dynamic value = config.arguments[key];
      if (value == null || value is bool) {
        buildArguments.add('--$key');
      } else if (value is Map) {
        for (String subKey in value.keys) {
          buildArguments.addAll(['--$key', '$subKey=${value[subKey]}']);
        }
      } else {
        buildArguments.addAll(['--$key', value]);
      }
    }

    if (!config.arguments.containsKey('build-name')) {
      buildArguments.addAll(['--build-name', appBuildName]);
    }
    if (!config.arguments.containsKey('build-number')) {
      buildArguments.addAll(['--build-number', appBuildNumber]);
    }

    ProcessResult processResult = await flutter.withEnv(environment).build(
      [buildSubcommand, ...buildArguments],
    );

    if (processResult.exitCode != 0) {
      throw BuildError('${processResult.stderr}');
    }

    return resultResolver.resolve(config)..duration = time.elapsed;
  }
}
