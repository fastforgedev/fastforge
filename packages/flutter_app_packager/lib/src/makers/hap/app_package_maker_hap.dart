import 'dart:io';

import 'package:flutter_app_packager/src/api/app_package_maker.dart';

class AppPackageMakerHap extends AppPackageMaker {
  @override
  String get name => 'hap';
  @override
  String get platform => 'ohos';
  @override
  String get packageFormat => 'hap';

  @override
  Future<MakeResult> make(MakeConfig config) {
    File pkgFile = config.buildOutputFiles.first;
    pkgFile.copySync(config.outputFile.path);
    return Future.value(resultResolver.resolve(config));
  }
}
