import 'dart:io';

import 'package:pub_semver/pub_semver.dart' show Version;
import 'package:pubspec_parse/pubspec_parse.dart';
import 'package:shell_executor/shell_executor.dart';

export 'package:pub_semver/pub_semver.dart' show Version;

extension VersionExtension on Version {
  /// Version core, without pre-release and build metadata
  String get versionCore => '$major.$minor.$patch';
}

typedef PublishProgressCallback = void Function(int sent, int total);

abstract class AppPackagePublisher {
  List<Command> get requirements => [];

  String get name => throw UnimplementedError();
  List<String> get supportedPlatforms => throw UnimplementedError();

  Future<PublishResult> publish(
    FileSystemEntity fileSystemEntity, {
    Map<String, String>? environment,
    Map<String, dynamic>? publishArguments,
    PublishProgressCallback? onPublishProgress,
  });
}

class PublishConfig {
  PublishConfig({
    String? appVersion,
  }) {
    if (appVersion != null) {
      this.appVersion = Version.parse(appVersion);
    } else {
      // TODO(lijy91): Remove this fallback when we have a better way to get the app version
      final pubspecFile = File('pubspec.yaml');
      if (pubspecFile.existsSync()) {
        final pubspec = Pubspec.parse(pubspecFile.readAsStringSync());
        this.appVersion = pubspec.version;
      }
    }
  }

  /// The version of the app
  late final Version? appVersion;
}

class PublishResult {
  PublishResult({
    this.url,
  });

  final String? url;
}

class PublishError extends Error {
  PublishError([this.message]);
  final String? message;

  @override
  String toString() {
    var message = this.message;
    return (message != null) ? 'PublishError: $message' : 'PublishError';
  }
}
