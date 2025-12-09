import 'dart:io';

import 'package:flutter_app_publisher/src/api/app_package_publisher.dart';

const kEnvMinioEndpoint = 'MINIO_ENDPOINT';
const kEnvMinioAccessKey = 'MINIO_ACCESS_KEY';
const kEnvMinioSecretKey = 'MINIO_SECRET_KEY';

class PublishMinioConfig extends PublishConfig {
  PublishMinioConfig({
    required this.endPoint,
    required this.accessKey,
    required this.secretKey,
    this.region,
    required this.bucket,
    this.savekeyPrefix,
  });

  factory PublishMinioConfig.parse(
    Map<String, String>? environment,
    Map<String, dynamic>? publishArguments,
  ) {
    final env = environment ?? Platform.environment;

    String? endPoint = publishArguments?['endpoint'];
    String? accessKey = publishArguments?['access-key'];
    String? secretKey = publishArguments?['secret-key'];
    String? region = publishArguments?['region'];
    String? bucket = publishArguments?['bucket'];
    String? savekeyPrefix = publishArguments?['savekey-prefix'];

    if ((endPoint ?? '').isEmpty) endPoint = env[kEnvMinioEndpoint];
    if ((accessKey ?? '').isEmpty) accessKey = env[kEnvMinioAccessKey];
    if ((secretKey ?? '').isEmpty) secretKey = env[kEnvMinioSecretKey];

    if ((endPoint ?? '').isEmpty) {
      throw PublishError(
        'Minio endpoint is required. '
        'Please provide it via `--minio-endpoint` argument or `$kEnvMinioEndpoint` environment variable.',
      );
    }
    if ((accessKey ?? '').isEmpty) {
      throw PublishError(
        'Minio access key is required. '
        'Please provide it via `--minio-access-key` argument or `$kEnvMinioAccessKey` environment variable.',
      );
    }
    if ((secretKey ?? '').isEmpty) {
      throw PublishError(
        'Minio secret key is required. '
        'Please provide it via `--minio-secret-key` argument or `$kEnvMinioSecretKey` environment variable.',
      );
    }
    if ((bucket ?? '').isEmpty) {
      throw PublishError(
        'Minio bucket is required. '
        'Please provide it via `--minio-bucket` argument.',
      );
    }
    return PublishMinioConfig(
      endPoint: endPoint!,
      accessKey: accessKey!,
      secretKey: secretKey!,
      region: region,
      bucket: bucket!,
      savekeyPrefix: savekeyPrefix,
    );
  }

  final String endPoint;
  final String accessKey;
  final String secretKey;
  final String? region;
  final String bucket;
  String? savekeyPrefix;
}
