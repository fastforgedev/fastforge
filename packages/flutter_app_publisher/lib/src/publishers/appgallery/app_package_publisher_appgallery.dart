import 'dart:io';

import 'package:dio/dio.dart';
import 'package:flutter_app_publisher/flutter_app_publisher.dart';
import 'package:flutter_app_publisher/src/publishers/appgallery/publish_appgallery_config.dart';

/// Huawei AppGallery Connect publishing API doc
/// [https://developer.huawei.com/consumer/cn/doc/app/agc-help-test-api-add-test-package-0000002236201330]
class AppPackagePublisherAppGallery extends AppPackagePublisher {
  final Dio _dio = Dio();

  @override
  String get name => 'appgallery';

  @override
  List<String> get supportedPlatforms => ['ohos'];

  @override
  Future<PublishResult> publish(
    FileSystemEntity fileSystemEntity, {
    Map<String, String>? environment,
    Map<String, dynamic>? publishArguments,
    PublishProgressCallback? onPublishProgress,
  }) async {
    File file = fileSystemEntity as File;
    PublishAppGalleryConfig publishConfig = PublishAppGalleryConfig.parse(
      environment,
      publishArguments,
    );

    try {
      String fileName = file.uri.pathSegments.last;

      // Get access token (1/4)
      String accessToken = await getAccessToken(
        publishConfig.clientId,
        publishConfig.clientSecret,
      );

      // Get upload URL (2/4)
      Map<String, dynamic> uploadUrlInfo = await getUploadUrl(
        publishConfig.clientId,
        accessToken,
        publishConfig.appId,
        fileName,
        file.lengthSync(),
      );

      // Upload file (3/4)
      await uploadFile(
        uploadUrlInfo,
        file,
        onPublishProgress,
      );

      // Apply Package Info (4/4)
      await applyUpload(
        publishConfig.clientId,
        accessToken,
        publishConfig.appId,
        fileName,
        uploadUrlInfo['objectId'],
      );

      return PublishResult(
        url:
            'https://developer.huawei.com/consumer/cn/service/josp/agc/index.html',
      );
    } catch (e) {
      throw PublishError(e.toString());
    }
  }

  Future<String> getAccessToken(
    String clientId,
    String clientSecret,
  ) async {
    Map<String, dynamic> data = {
      'grant_type': 'client_credentials',
      'client_id': clientId,
      'client_secret': clientSecret,
    };
    try {
      Response response = await _dio.post(
        'https://connect-api.cloud.huawei.com/api/oauth2/v1/token',
        data: data,
      );
      if (response.statusCode == 200 && response.data['access_token'] != null) {
        return response.data['access_token'];
      } else {
        throw PublishError('getAccessToken error: ${response.data}');
      }
    } catch (e) {
      throw PublishError(e.toString());
    }
  }

  Future<Map<String, dynamic>> getUploadUrl(
    String clientId,
    String accessToken,
    String appId,
    String fileName,
    int contentLength,
  ) async {
    Map<String, dynamic> query = {
      'appId': appId,
      'fileName': fileName,
      'contentLength': contentLength,
    };
    try {
      Response response = await _dio.get(
        'https://connect-api.cloud.huawei.com/api/publish/v2/upload-url/for-obs',
        queryParameters: query,
        options: Options(
          headers: {
            'client_id': clientId,
            'Authorization': 'Bearer $accessToken',
            'Content-Type': 'application/json',
          },
        ),
      );
      if (response.data?['ret']?['code'] == 0) {
        return Map<String, dynamic>.from(response.data['urlInfo']);
      } else {
        throw PublishError('getUploadUrl error: ${response.data}');
      }
    } catch (e) {
      throw PublishError(e.toString());
    }
  }

  Future<void> uploadFile(
    Map<String, dynamic> urlInfo,
    File file,
    PublishProgressCallback? onPublishProgress,
  ) async {
    try {
      Response response = await _dio.put(
        urlInfo['url'] as String,
        data: file.openRead(),
        options: Options(
          headers: {
            ...Map<String, String>.from(urlInfo['headers']),
            'Content-Length': file.lengthSync().toString(),
          },
        ),
        onSendProgress: (count, total) {
          onPublishProgress?.call(count, total);
        },
      );
      if (response.statusCode != 200) {
        throw PublishError('uploadFile error: ${response.data}');
      }
    } catch (e) {
      throw PublishError('uploadFile error: ${e.toString()}');
    }
  }

  Future<Map<String, dynamic>> applyUpload(
    String clientId,
    String accessToken,
    String appId,
    String fileName,
    String objectId,
  ) async {
    Map<String, dynamic> headers = {
      'client_id': clientId,
      'Authorization': 'Bearer $accessToken',
    };
    Map<String, dynamic> query = {
      'appId': appId,
      'releaseType': 1,
      'releasePhase': 0,
    };
    Map<String, dynamic> data = {
      'fileName': fileName,
      'objectId': objectId,
    };
    try {
      Response response = await _dio.put(
        'https://connect-api.cloud.huawei.com/api/publish/v3/app-package-info',
        queryParameters: query,
        data: data,
        options: Options(headers: headers),
      );
      if (response.statusCode == 200 && response.data['ret']['code'] == 0) {
        return Map<String, dynamic>.from(response.data);
      } else {
        throw PublishError('applyUpload error: ${response.data}');
      }
    } catch (e) {
      throw PublishError('applyUpload error: ${e.toString()}');
    }
  }
}
