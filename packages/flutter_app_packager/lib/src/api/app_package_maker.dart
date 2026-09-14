import 'dart:convert';
import 'dart:io';

import 'package:flutter_app_packager/src/api/make_config.dart';
import 'package:flutter_app_packager/src/api/make_result.dart';
import 'package:shell_executor/shell_executor.dart';
import 'package:yaml/yaml.dart';

export 'make_config.dart';
export 'make_error.dart';
export 'make_result.dart';

Map<String, dynamic> _deepMerge(
  Map<String, dynamic> target,
  Map<String, dynamic> source,
) {
  for (final key in source.keys) {
    final value = source[key];
    if (value is Map<String, dynamic> &&
        target.containsKey(key) &&
        target[key] is Map<String, dynamic>) {
      target[key] = _deepMerge(target[key] as Map<String, dynamic>, value);
    } else {
      target[key] = value;
    }
  }
  return target;
}

Map<String, dynamic> loadMakeConfigYaml(String path) {
  Map<String, dynamic> config = {};

  final file = File(path);
  final parentDir = file.parent.parent;
  final defaultFile = File('${parentDir.path}/make_config.yaml');

  bool defaultExists = defaultFile.existsSync();
  bool fileExists = file.existsSync();

  if (!defaultExists && !fileExists) {
    throw FileSystemException(
      'Neither the specific config file nor the default config file exists.',
      path,
    );
  }

  if (defaultExists) {
    final yamlDoc = loadYaml(defaultFile.readAsStringSync());
    if (yamlDoc != null) {
      final decoded = json.decode(json.encode(yamlDoc));
      if (decoded is Map<String, dynamic>) {
        config = _deepMerge(config, decoded);
      } else {
        throw const FormatException(
          'Default config file is not a valid YAML map.',
        );
      }
    } else {
      throw const FormatException(
        'Default config file is not a valid YAML map.',
      );
    }
  }

  if (fileExists) {
    final yamlDoc = loadYaml(file.readAsStringSync());
    if (yamlDoc != null) {
      final decoded = json.decode(json.encode(yamlDoc));
      if (decoded is Map<String, dynamic>) {
        config = _deepMerge(config, decoded);
      } else {
        throw const FormatException(
          'Specific config file is not a valid YAML map.',
        );
      }
    } else {
      throw const FormatException(
        'Specific config file is not a valid YAML map.',
      );
    }
  }

  return config;
}

abstract class AppPackageMaker {
  List<Command> get requirements => [];

  String get name => throw UnimplementedError();
  String get platform => throw UnimplementedError();
  bool get isSupportedOnCurrentPlatform => true;
  String get packageFormat => throw UnimplementedError();

  MakeConfigLoader get configLoader {
    return DefaultMakeConfigLoader()
      ..platform = platform
      ..packageFormat = packageFormat;
  }

  MakeResultResolver get resultResolver => DefaultMakeResultResolver();

  bool match(String platform, [String? target]) {
    return this.platform == platform && name == target;
  }

  Future<MakeResult> make(MakeConfig config) {
    throw UnimplementedError();
  }
}
