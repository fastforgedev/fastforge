import 'dart:io';

import 'package:flutter_app_publisher/src/api/app_package_publisher.dart';
import 'package:flutter_app_publisher/src/publishers/minio/publish_minio_config.dart';
import 'package:minio/io.dart';
import 'package:minio/minio.dart';

class AppPackagePublisherMinio extends AppPackagePublisher {
  @override
  String get name => 'minio';

  @override
  List<String> get supportedPlatforms => [
        'android',
        'ios',
        'linux',
        'macos',
        'windows',
        'web',
      ];

  @override
  Future<PublishResult> publish(
    FileSystemEntity fileSystemEntity, {
    Map<String, String>? environment,
    Map<String, dynamic>? publishArguments,
    PublishProgressCallback? onPublishProgress,
  }) async {
    File file = fileSystemEntity as File;
    PublishMinioConfig publishConfig = PublishMinioConfig.parse(
      environment,
      publishArguments,
    );

    try {
      final minioClient = Minio(
        endPoint: publishConfig.endPoint,
        accessKey: publishConfig.accessKey,
        secretKey: publishConfig.secretKey,
        region: publishConfig.region,
        useSSL: true,
      );

      String saveKey = file.path.split('/').last;
      if ((publishConfig.savekeyPrefix ?? '').isNotEmpty) {
        saveKey = '${publishConfig.savekeyPrefix}/$saveKey';
      }

      int total = file.lengthSync();

      await minioClient.fPutObject(
        publishConfig.bucket,
        saveKey,
        file.path,
        onProgress: (int sent) {
          if (onPublishProgress != null) {
            onPublishProgress(sent, total);
          }
        },
      );
      return PublishResult(
        url: 'https://${publishConfig.endPoint}/$saveKey',
      );
    } on MinioError catch (error) {
      throw PublishError('${error.message}');
    } catch (error) {
      rethrow;
    }
  }
}
