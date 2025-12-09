import 'dart:io';

import 'package:flutter_app_publisher/flutter_app_publisher.dart';

const kEnvAppGalleryClientId = 'APP_GALLERY_CLIENT_ID';
const kEnvAppGalleryClientSecret = 'APP_GALLERY_CLIENT_SECRET';

class PublishAppGalleryConfig extends PublishConfig {
  PublishAppGalleryConfig({
    required this.clientId,
    required this.clientSecret,
    required this.appId,
  });

  factory PublishAppGalleryConfig.parse(
    Map<String, String>? environment,
    Map<String, dynamic>? publishArguments,
  ) {
    String? clientId =
        (environment ?? Platform.environment)[kEnvAppGalleryClientId];
    if ((clientId ?? '').isEmpty) {
      throw PublishError(
        'Missing `$kEnvAppGalleryClientId` environment variable.',
      );
    }

    String? clientSecret =
        (environment ?? Platform.environment)[kEnvAppGalleryClientSecret];
    if ((clientSecret ?? '').isEmpty) {
      throw PublishError(
        'Missing `$kEnvAppGalleryClientSecret` environment variable.',
      );
    }

    String? appId = publishArguments?['app-id'];
    if ((appId ?? '').isEmpty) {
      throw PublishError('Missing `app-id` arg');
    }

    return PublishAppGalleryConfig(
      clientId: clientId!,
      clientSecret: clientSecret!,
      appId: appId!,
    );
  }

  final String clientId;
  final String clientSecret;
  final String appId;
}
