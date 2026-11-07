import 'dart:io';

import 'package:flutter_app_packager/src/makers/appimage/app_package_maker_appimage.dart';
import 'package:flutter_app_packager/src/makers/appimage/make_appimage_config.dart';
import 'package:shell_executor/shell_executor.dart';
import 'package:test/test.dart';

class _AppImageExecutor extends ShellExecutor {
  bool invoked = false;

  @override
  Future<ProcessResult> exec(
    String executable,
    List<String> arguments, {
    String? workingDirectory,
    Map<String, String>? environment,
  }) async {
    if (executable == 'ldd') {
      return ProcessResult(0, 0, '', '');
    }
    if (executable == 'appimagetool') {
      invoked = true;
      // appimagetool must detect the architecture from the AppDir instead of
      // embedding an x86_64 runtime into an aarch64 application (#380).
      expect(environment?.containsKey('ARCH') ?? false, isFalse);
      expect(arguments.first, '--no-appstream');
      expect(Directory(arguments[1]).existsSync(), isTrue);
      File(arguments.last).writeAsStringSync('appimage');
      return ProcessResult(0, 0, '', '');
    }
    return super.exec(
      executable,
      arguments,
      workingDirectory: workingDirectory,
      environment: environment,
    );
  }
}

void main() {
  test(
    'AppImage leaves architecture detection to appimagetool',
    () async {
      final current = Directory.current;
      final originalExecutor = ShellExecutor.global;
      final temp = Directory.systemTemp.createTempSync('fastforge_appimage_');
      final executor = _AppImageExecutor();
      try {
        Directory.current = temp;
        ShellExecutor.global = executor;
        File('pubspec.yaml')
            .writeAsStringSync('name: demo\nversion: 1.0.0+1\n');
        final bundle = Directory('bundle')..createSync();
        Directory('bundle/lib').createSync();
        File('bundle/demo').writeAsStringSync('binary');
        File('icon.png').writeAsStringSync('icon');
        final config = MakeAppImageConfig(displayName: 'Demo', icon: 'icon.png')
          ..platform = 'linux'
          ..packageFormat = 'appimage'
          ..buildMode = 'release'
          ..buildOutputDirectory = bundle
          ..buildOutputFiles = []
          ..outputDirectory = Directory('dist/');

        final result = await AppPackageMakerAppImage().make(config);

        expect(executor.invoked, isTrue);
        expect(result.artifacts.single.path, endsWith('.AppImage'));
        expect(
            File(result.artifacts.single.path).readAsStringSync(), 'appimage');
      } finally {
        ShellExecutor.global = originalExecutor;
        Directory.current = current;
        temp.deleteSync(recursive: true);
      }
    },
    skip: Platform.isWindows ? 'AppImage packaging uses Unix tools' : false,
  );
}
