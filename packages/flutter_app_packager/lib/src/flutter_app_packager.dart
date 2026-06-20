import 'dart:io';

import 'package:flutter_app_packager/src/api/app_package_maker.dart';
import 'package:flutter_app_packager/src/makers/makers.dart';
import 'package:flutter_app_packager/src/makers/pacman/app_package_maker_pacman.dart';
import 'package:shell_executor/shell_executor.dart';

class FlutterAppPackager {
  final List<AppPackageMaker> _makers = [
    AppPackageMakerAab(),
    AppPackageMakerApk(),
    AppPackageMakerApp(),
    AppPackageMakerAppImage(),
    AppPackageMakerCustom('android'),
    AppPackageMakerCustom('ios'),
    AppPackageMakerCustom('linux'),
    AppPackageMakerCustom('macos'),
    AppPackageMakerCustom('ohos'),
    AppPackageMakerCustom('web'),
    AppPackageMakerCustom('windows'),
    AppPackageMakerDeb(),
    AppPackageMakerDirect('linux'),
    AppPackageMakerDirect('windows'),
    AppPackageMakerDirect('web'),
    AppPackageMakerDmg(),
    AppPackageMakerExe(),
    AppPackageMakerHap(),
    AppPackageMakerIpa(),
    AppPackageMakerMsix(),
    AppPackageMakerPkg(),
    AppPackageMakerRPM(),
    AppPackageMakerPacman(),
    AppPackageMakerZip('linux'),
    AppPackageMakerZip('macos'),
    AppPackageMakerZip('windows'),
    AppPackageMakerZip('web'),
  ];

  Future<MakeResult> package(
    String platform,
    String target,
    Map<String, dynamic>? arguments,
    Directory outputDirectory, {
    required Directory buildOutputDirectory,
    required List<File> buildOutputFiles,
  }) async {
    final maker = _makers.firstWhere((e) => e.match(platform, target));
    final config = maker.configLoader.load(
      arguments,
      outputDirectory,
      buildOutputDirectory: buildOutputDirectory,
      buildOutputFiles: buildOutputFiles,
    );

    // Run prepackage hooks
    await _runHooks(config.prepackageHooks, config);

    final result = await maker.make(config);

    // Run postpackage hooks
    await _runHooks(config.postpackageHooks, config);

    return result;
  }

  /// Execute a list of shell hook commands.
  Future<void> _runHooks(List<String>? hooks, MakeConfig config) async {
    if (hooks == null || hooks.isEmpty) return;
    for (final hook in hooks) {
      final result = await $(
        'sh',
        ['-c', hook],
        environment: {
          'PLATFORM': config.platform,
          'PACKAGE_FORMAT': config.packageFormat,
          'BUILD_MODE': config.buildMode,
          'OUTPUT_DIRECTORY': config.outputDirectory.path,
          'BUILD_OUTPUT_DIRECTORY': config.buildOutputDirectory.path,
          'BUILD_OUTPUT_FILES':
              config.buildOutputFiles.map((f) => f.path).join(':'),
        },
      );
      if (result.exitCode != 0) {
        throw Exception(
          'Hook failed (exit ${result.exitCode}): $hook\n${result.stderr}',
        );
      }
    }
  }
}
