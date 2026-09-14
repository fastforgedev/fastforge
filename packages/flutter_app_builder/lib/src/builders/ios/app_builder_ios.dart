import 'dart:io';

import 'package:flutter_app_builder/src/build_error.dart';
import 'package:flutter_app_builder/src/build_result.dart';
import 'package:flutter_app_builder/src/builders/app_builder.dart';
import 'package:flutter_app_builder/src/builders/ios/build_ios_result.dart';

class AppBuilderIos extends AppBuilder {
  @override
  String get platform => 'ios';

  @override
  bool get isSupportedOnCurrentPlatform => Platform.isMacOS;

  @override
  BuildResultResolver get resultResolver => BuildIosResultResolver();

  String _buildSubcommand = 'ipa';

  @override
  String get buildSubcommand => _buildSubcommand;

  @override
  Future<BuildResult> build({
    required Map<String, dynamic> arguments,
    Map<String, String>? environment,
  }) {
    if (arguments.containsKey('no-codesign')) {
      _buildSubcommand = 'ios';
    } else {
      _buildSubcommand = 'ipa';
      if (!arguments.containsKey('export-options-plist') &&
          !arguments.containsKey('export-method')) {
        throw BuildError(
          'Missing `export-options-plist` or `export-method` build argument.',
        );
      }
    }
    return super.build(
      arguments: arguments,
      environment: environment,
    );
  }
}
