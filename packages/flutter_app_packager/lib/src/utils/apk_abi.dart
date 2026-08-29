import 'package:path/path.dart' as path;

String abiFromApkPath(String apkPath, String buildMode) {
  // basename is like app<-arch>-<mode>.apk
  final baseName = path.basename(apkPath);
  const prefix = 'app';
  final suffix = '-$buildMode.apk';
  final arch = baseName.substring(
    prefix.length,
    baseName.length - suffix.length,
  );
  return arch;
}
