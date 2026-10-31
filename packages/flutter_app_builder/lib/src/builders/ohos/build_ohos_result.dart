import 'dart:io';

import 'package:flutter_app_builder/src/build_config.dart';
import 'package:flutter_app_builder/src/build_result.dart';
import 'package:glob/glob.dart';
import 'package:glob/list_local_fs.dart';

class BuildOhosResultResolver extends BuildResultResolver {
  BuildOhosResultResolver(
    this.target,
  ) : _actualResultResolver = target == 'hap'
            ? _BuildOhosHapResultResolver()
            : _BuildOhosAppResultResolver();

  factory BuildOhosResultResolver.hap() {
    return BuildOhosResultResolver('hap');
  }

  factory BuildOhosResultResolver.app() {
    return BuildOhosResultResolver('app');
  }

  final String target;

  late BuildResultResolver _actualResultResolver;

  @override
  BuildResult resolve(BuildConfig config) {
    return _actualResultResolver.resolve(config);
  }
}

class BuildOhosResult extends BuildResult {
  BuildOhosResult(this.target, BuildConfig config)
      : _actualResult = target == 'hap'
            ? _BuildOhosHapResult(config)
            : _BuildOhosAppResult(config),
        super(config);

  factory BuildOhosResult.hap(BuildConfig config) {
    return BuildOhosResult('hap', config);
  }

  factory BuildOhosResult.app(BuildConfig config) {
    return BuildOhosResult('app', config);
  }

  final String target;

  late BuildResult _actualResult;

  @override
  Directory get outputDirectory => _actualResult.outputDirectory;
}

class _BuildOhosHapResultResolver extends BuildResultResolver {
  @override
  BuildResult resolve(BuildConfig config) {
    final r = _BuildOhosHapResult(config);
    r.outputFiles = _resolveOutputFiles(config, r.outputDirectory, 'hap');
    return r;
  }
}

class _BuildOhosHapResult extends BuildResult {
  _BuildOhosHapResult(BuildConfig config) : super(config);

  @override
  Directory get outputDirectory {
    String flavor = config.flavor ?? 'default';
    String path = 'ohos/entry/build/$flavor/outputs/$flavor';
    return Directory(path);
  }
}

class _BuildOhosAppResultResolver extends BuildResultResolver {
  @override
  BuildResult resolve(BuildConfig config) {
    final r = _BuildOhosAppResult(config);
    r.outputFiles = _resolveOutputFiles(config, r.outputDirectory, 'app');
    return r;
  }
}

List<File> _resolveOutputFiles(
  BuildConfig config,
  Directory outputDirectory,
  String extension,
) {
  // Packagers use the first output file. Prefer signed artifacts unless signing
  // was explicitly disabled, and fall back to unsigned builds when necessary.
  final signingStates = config.arguments.containsKey('no-codesign')
      ? ['unsigned']
      : ['signed', 'unsigned'];
  for (final signingState in signingStates) {
    final pattern = [
      '${outputDirectory.path}/**',
      '-${config.flavor ?? 'default'}',
      '-$signingState.$extension',
    ].join();
    final files = Glob(pattern).listSync().map((e) => File(e.path)).toList();
    if (files.isNotEmpty) return files;
  }
  return [];
}

class _BuildOhosAppResult extends BuildResult {
  _BuildOhosAppResult(BuildConfig config) : super(config);

  @override
  Directory get outputDirectory {
    String flavor = config.flavor ?? 'default';
    String path = 'ohos/build/outputs/$flavor';
    return Directory(path);
  }
}
