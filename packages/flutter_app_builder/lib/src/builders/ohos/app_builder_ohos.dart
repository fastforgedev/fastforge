import 'package:flutter_app_builder/src/build_result.dart';
import 'package:flutter_app_builder/src/builders/app_builder.dart';
import 'package:flutter_app_builder/src/builders/ohos/build_ohos_result.dart';

class AppBuilderOhos extends AppBuilder {
  AppBuilderOhos(this.target);

  factory AppBuilderOhos.hap() {
    return AppBuilderOhos('hap');
  }

  factory AppBuilderOhos.app() {
    return AppBuilderOhos('app');
  }

  @override
  String get platform => 'ohos';

  @override
  bool get isSupportedOnCurrentPlatform => true;

  @override
  BuildResultResolver get resultResolver => BuildOhosResultResolver(target);

  @override
  String get buildSubcommand => target == 'hap' ? 'hap' : 'app';

  final String target;

  @override
  bool match(String platform, [String? target]) {
    return this.platform == platform && this.target == target;
  }
}
