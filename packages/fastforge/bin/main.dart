import 'dart:io';

import 'package:fastforge/fastforge.dart';

Future<void> main(List<String> args) async {
  try {
    final cli = UnifiedDistributorCommandLineInterface(
      'fastforge',
      'Package and publish your apps with ease.',
      packageName: 'fastforge',
      displayName: 'Fastforge',
    );
    await cli.run(args);
  } catch (error, stackTrace) {
    stderr.writeln(error.toString());
    stderr.writeln(stackTrace.toString());
    exit(1);
  }
}
