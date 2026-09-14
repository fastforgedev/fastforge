import 'dart:io';

import 'package:flutter_app_packager/src/api/app_package_maker.dart';
import 'package:shell_executor/shell_executor.dart';

class AppPackageMakerIpa extends AppPackageMaker {
  @override
  String get name => 'ipa';
  @override
  String get platform => 'ios';
  @override
  String get packageFormat => 'ipa';

  @override
  Future<MakeResult> make(MakeConfig config) async {
    final pkgFile = config.buildOutputFiles.first;
    final isApp = pkgFile.path.endsWith('.app');
    if (isApp) {
      // pkgFilePath build/ios/iphoneos/Runner.app
      final iphoneosDirectory = pkgFile.parent;
      final iosDirectory = iphoneosDirectory.parent;
      final payloadDirectory = Directory('${iosDirectory.path}/Payload');
      final ipaFile = File('${iphoneosDirectory.path}/Runner.ipa');
      try {
        await $('cp', ['-RH', iphoneosDirectory.path, payloadDirectory.path]);
        await $(
          'zip',
          [
            '-r9',
            ipaFile.path.replaceFirst('${iosDirectory.path}/', ''),
            'Payload',
          ],
          workingDirectory: iosDirectory.path,
        );
      } catch (error) {
        rethrow;
      } finally {
        payloadDirectory.deleteSync(recursive: true);
      }
      ipaFile.copySync(config.outputFile.path);
    } else {
      pkgFile.copySync(config.outputFile.path);
    }
    return Future.value(resultResolver.resolve(config));
  }
}
