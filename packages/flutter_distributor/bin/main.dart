import 'dart:io';

import 'package:fastforge/fastforge.dart';

Future<void> main(List<String> args) async {
  try {
    final cli = UnifiedDistributorCommandLineInterface(
      'flutter_distributor',
      'Package and publish your apps with ease.',
      packageName: 'flutter_distributor',
      displayName: 'Flutter Distributor',
    );
    await cli.run(args);
  } catch (error, stackTrace) {
    stderr.writeln(error.toString());
    stderr.writeln(stackTrace.toString());
    exit(1);
  }
}
