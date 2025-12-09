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
    final String pattern = [
      '${r.outputDirectory.path}/**',
      '-${config.flavor ?? 'default'}',
      '-signed.hap',
    ].join();
    r.outputFiles = Glob(pattern).listSync().map((e) => File(e.path)).toList();
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
    final String pattern = [
      '${r.outputDirectory.path}/**',
      '-${config.flavor ?? 'default'}',
      '-signed.app',
    ].join();
    r.outputFiles = Glob(pattern).listSync().map((e) => File(e.path)).toList();
    return r;
  }
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
