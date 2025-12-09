import 'package:flutter_app_builder/src/build_result.dart';
import 'package:flutter_app_builder/src/builders/builders.dart';
import 'package:flutter_app_builder/src/commands/flutter.dart';

class FlutterAppBuilder {
  final List<AppBuilder> _builders = [
    AppBuilderAndroid.aab(),
    AppBuilderAndroid.apk(),
    AppBuilderIos(),
    AppBuilderLinux(),
    AppBuilderMacOs(),
    AppBuilderOhos.hap(),
    AppBuilderOhos.app(),
    AppBuilderWeb(),
    AppBuilderWindows(),
  ];

  Future<void> clean({
    Map<String, String>? environment,
  }) async {
    await flutter.withEnv(environment).clean();
  }

  Future<BuildResult> build(
    String platform, {
    String? target,
    required Map<String, dynamic> arguments,
    Map<String, String>? environment,
  }) {
    final builder = _builders.firstWhere((e) => e.match(platform, target));
    if (!builder.isSupportedOnCurrentPlatform) {
      throw UnsupportedError(
        '${builder.runtimeType} is not supported on the current platform',
      );
    }
    return builder.build(
      arguments: arguments,
      environment: environment,
    );
  }
}
