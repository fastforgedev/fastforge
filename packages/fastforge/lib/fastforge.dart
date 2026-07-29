library fastforge;

import 'package:fastforge/src/unified_distributor.dart';

export 'src/check_version_result.dart';
export 'src/cli/cli.dart';
export 'src/distribute_options.dart';
export 'src/extensions/string.dart';
export 'src/unified_distributor.dart';
export 'src/utils/default_shell_executor.dart';
export 'src/utils/logger.dart';

/// The main class for the Fastforge package.
///
/// This class extends the [UnifiedDistributor] class and provides a
/// default implementation for the [UnifiedDistributor] class.
class Fastforge extends UnifiedDistributor {
  /// Creates a new instance of the Fastforge class.
  Fastforge() : super('fastforge', 'Fastforge');
}
