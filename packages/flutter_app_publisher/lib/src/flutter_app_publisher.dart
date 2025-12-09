import 'dart:io';

import 'package:flutter_app_publisher/src/api/app_package_publisher.dart';
import 'package:flutter_app_publisher/src/publishers/publishers.dart';

class FlutterAppPublisher {
  final List<AppPackagePublisher> _publishers = [
    AppPackagePublisherAppCenter(),
    AppPackagePublisherAppGallery(),
    AppPackagePublisherAppStore(),
    AppPackagePublisherFir(),
    AppPackagePublisherFirebase(),
    AppPackagePublisherFirebaseHosting(),
    AppPackagePublisherGithub(),
    AppPackagePublisherMinio(),
    AppPackagePublisherPgyer(),
    AppPackagePublisherPlayStore(),
    AppPackagePublisherQiniu(),
    AppPackagePublisherVercel(),
  ];

  Future<PublishResult> publish(
    FileSystemEntity fileSystemEntity, {
    required String target,
    Map<String, String>? environment,
    Map<String, dynamic>? publishArguments,
    PublishProgressCallback? onPublishProgress,
  }) async {
    AppPackagePublisher publisher = _publishers.firstWhere(
      (e) => e.name == target,
    );
    return await publisher.publish(
      fileSystemEntity,
      environment: environment,
      publishArguments: publishArguments,
      onPublishProgress: onPublishProgress,
    );
  }
}
