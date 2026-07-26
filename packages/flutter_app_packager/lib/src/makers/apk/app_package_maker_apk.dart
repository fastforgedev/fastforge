import 'package:flutter_app_packager/src/api/app_package_maker.dart';
import 'package:flutter_app_packager/src/utils/apk_abi.dart';

class AppPackageMakerApk extends AppPackageMaker {
  @override
  String get name => 'apk';
  @override
  String get platform => 'android';
  @override
  String get packageFormat => 'apk';

  @override
  Future<MakeResult> make(MakeConfig config) {
    if (config.splitPerAbi == true) {
      for (final outfile in config.buildOutputFiles) {
        final abi = abiFromApkPath(outfile.path, config.buildMode);
        outfile.copySync(
          config.outputFile.path.replaceFirst('.apk', '$abi.apk'),
        );
      }
    } else {
      config.buildOutputFiles.first.copySync(config.outputFile.path);
    }

    return Future.value(resultResolver.resolve(config));
  }
}
