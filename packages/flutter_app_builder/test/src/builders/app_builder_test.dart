import 'dart:io';

import 'package:flutter_app_builder/src/build_config.dart';
import 'package:flutter_app_builder/src/build_result.dart';
import 'package:flutter_app_builder/src/builders/app_builder.dart';
import 'package:shell_executor/shell_executor.dart';
import 'package:test/test.dart';

void main() {
  late Directory workingDirectory;
  late String originalWorkingDirectory;
  late ShellExecutor originalShellExecutor;
  late _RecordingShellExecutor shellExecutor;

  setUp(() {
    workingDirectory = Directory.systemTemp.createTempSync(
      'flutter_app_builder_test.',
    );
    File('${workingDirectory.path}/pubspec.yaml').writeAsStringSync('''
name: example
version: 1.2.3+4
''');
    originalWorkingDirectory = Directory.current.path;
    Directory.current = workingDirectory;

    originalShellExecutor = ShellExecutor.global;
    shellExecutor = _RecordingShellExecutor();
    ShellExecutor.global = shellExecutor;
  });

  tearDown(() {
    ShellExecutor.global = originalShellExecutor;
    Directory.current = originalWorkingDirectory;
    workingDirectory.deleteSync(recursive: true);
  });

  test('passes pubspec version using Flutter build flags', () async {
    await _TestAppBuilder().build(
      arguments: {
        'dart-define': {'APP_ENV': 'test'},
      },
    );

    expect(shellExecutor.arguments, [
      'build',
      'windows',
      '--dart-define',
      'APP_ENV=test',
      '--build-name',
      '1.2.3',
      '--build-number',
      '4',
    ]);
    expect(
      shellExecutor.arguments,
      isNot(contains('FLUTTER_BUILD_NAME=1.2.3')),
    );
    expect(
      shellExecutor.arguments,
      isNot(contains('FLUTTER_BUILD_NUMBER=4')),
    );
  });

  test('does not override explicitly supplied build flags', () async {
    await _TestAppBuilder().build(
      arguments: {
        'build-name': '2.0.0',
        'build-number': '9',
      },
    );

    expect(shellExecutor.arguments, [
      'build',
      'windows',
      '--build-name',
      '2.0.0',
      '--build-number',
      '9',
    ]);
  });
}

class _TestAppBuilder extends AppBuilder {
  @override
  String get platform => 'windows';

  @override
  bool get isSupportedOnCurrentPlatform => true;

  @override
  BuildResultResolver get resultResolver => _TestBuildResultResolver();
}

class _TestBuildResultResolver extends BuildResultResolver {
  @override
  BuildResult resolve(BuildConfig config) => _TestBuildResult(config);
}

class _TestBuildResult extends BuildResult {
  _TestBuildResult(BuildConfig config) : super(config);

  @override
  Directory get outputDirectory => Directory.current;
}

class _RecordingShellExecutor extends ShellExecutor {
  List<String>? arguments;

  @override
  Future<ProcessResult> exec(
    String executable,
    List<String> arguments, {
    String? workingDirectory,
    Map<String, String>? environment,
  }) async {
    this.arguments = arguments;
    return ProcessResult(0, 0, '', '');
  }
}
