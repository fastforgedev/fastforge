import 'dart:io';

import 'package:flutter/widgets.dart';

import 'main.dart';

// Dedicated entry point for native packaging tests. A successful process must
// load the Flutter engine, AOT code and assets, and render its first frame.
Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  runApp(const MyApp());
  await WidgetsBinding.instance.waitUntilFirstFrameRasterized;
  const version = String.fromEnvironment('FASTFORGE_SMOKE_VERSION');
  stdout.writeln('fastforge-smoke:$version');
  await stdout.flush();
  exit(0);
}
