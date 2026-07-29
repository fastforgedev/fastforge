import 'package:args/command_runner.dart';
import 'package:fastforge/src/unified_distributor.dart';

/// Upgrade the current Fastforge-compatible CLI to the latest version.
class CommandUpgrade extends Command {
  CommandUpgrade(this.distributor);

  final UnifiedDistributor distributor;

  @override
  String get name => 'upgrade';

  @override
  String get description =>
      'Update ${distributor.displayName} to the latest version.';

  @override
  Future run() async {
    await distributor.upgrade();
  }
}
