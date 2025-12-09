import 'dart:io';
import 'dart:typed_data';

import 'package:dio/dio.dart';
import 'package:flutter_app_publisher/src/api/app_package_publisher.dart';
import 'package:flutter_app_publisher/src/publishers/github/publish_github_config.dart';

class AppPackagePublisherGithub extends AppPackagePublisher {
  final Dio _dio = Dio();

  @override
  String get name => 'github';

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
    File? file = fileSystemEntity is File ? fileSystemEntity : null;
    if (file == null) {
      throw PublishError(
        'The provided path "${fileSystemEntity.path}" is not a valid file.',
      );
    }
    PublishGithubConfig publishConfig = PublishGithubConfig.parse(
      environment,
      publishArguments,
    );
    // Set auth
    _dio.options = BaseOptions(
      headers: {
        'Authorization': 'token ${publishConfig.token}',
      },
    );

    // Get uploadUrl
    String? uploadUrl;
    if (publishConfig.releaseTitle?.isEmpty ?? true) {
      uploadUrl = await _getUploadurlByLatestRelease(publishConfig);
    } else {
      uploadUrl = await _getUploadurlByReleaseName(publishConfig);
      if (uploadUrl?.isEmpty ?? true) {
        uploadUrl = await _createRelease(publishConfig);
      }
    }
    if (uploadUrl?.isEmpty ?? true) {
      throw PublishError('Upload url isEmpty');
    }
    // Upload file
    String browserDownloadUrl =
        await _uploadReleaseAsset(file, uploadUrl!, onPublishProgress);
    return PublishResult(
      url: browserDownloadUrl,
    );
  }

  /// Get uploadUrl by releaseName
  Future<String?> _getUploadurlByReleaseName(
    PublishGithubConfig publishConfig,
  ) async {
    Response resp = await _dio.get(
      'https://api.github.com/repos/${publishConfig.repository}/releases',
    );
    List relist = (resp.data as List?) ?? [];
    var release = relist.firstWhere(
      (item) => item['name'] == publishConfig.releaseTitle,
      orElse: () => {},
    );
    return release?['upload_url'];
  }

  /// Create release
  Future<String?> _createRelease(PublishGithubConfig publishConfig) async {
    Response resp = await _dio.post(
      'https://api.github.com/repos/${publishConfig.repository}/releases',
      data: {
        'tag_name': publishConfig.releaseTitle,
        'name': publishConfig.releaseTitle,
        'draft': publishConfig.releaseDraft,
        'prerelease': publishConfig.releasePrerelease,
      },
    );
    return resp.data?['upload_url'];
  }

  /// Get uploadUrl by latest release
  Future<String?> _getUploadurlByLatestRelease(
    PublishGithubConfig publishConfig,
  ) async {
    Response resp = await _dio.get(
      'https://api.github.com/repos/${publishConfig.repository}/releases/latest',
    );
    return resp.data?['upload_url'];
  }

  /// Upload Release Asset
  Future<String> _uploadReleaseAsset(
    File file,
    String uploadUrl,
    PublishProgressCallback? onPublishProgress,
  ) async {
    // Fromat uploadUrl
    uploadUrl = uploadUrl.split('{').first;
    String fileName = file.path.split('/').last;
    Uint8List fileData = await file.readAsBytes();
    String url = '$uploadUrl?name=${Uri.encodeComponent(fileName)}';
    // dio upload
    _dio.options.contentType = 'application/octet-stream';
    _dio.options.headers
        .putIfAbsent(Headers.contentLengthHeader, () => fileData.length);
    String? browserDownloadUrl;
    try {
      Response resp = await _dio.post(
        url,
        data: Stream.fromIterable(fileData.map((e) => [e])),
        onSendProgress: (int sent, int total) {
          if (onPublishProgress != null) {
            onPublishProgress(sent, total);
          }
        },
      );
      browserDownloadUrl = resp.data?['browser_download_url'];
    } catch (e) {
      throw PublishError(e.toString());
    }
    // Check release asset
    if (browserDownloadUrl?.isEmpty ?? true) {
      throw PublishError('Release asset exist [$fileName]');
    }
    return browserDownloadUrl!;
  }
}
