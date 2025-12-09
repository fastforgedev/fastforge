import 'dart:io';
import 'dart:convert';

import 'package:dio/dio.dart';
import 'package:flutter_app_publisher/src/api/app_package_publisher.dart';
import 'package:flutter_app_publisher/src/publishers/pgyer/publish_pgyer_config.dart';

/// pgyer doc [https://www.pgyer.com/doc/view/api#fastUploadApp]
class AppPackagePublisherPgyer extends AppPackagePublisher {
  @override
  String get name => 'pgyer';

  // dio 网络请求实例
  final Dio _dio = Dio();
  // 轮询尝试次数
  int tryCount = 0;
  // 最大尝试轮询次数
  final maxTryCount = 10;

  @override
  Future<PublishResult> publish(
    FileSystemEntity fileSystemEntity, {
    Map<String, String>? environment,
    Map<String, dynamic>? publishArguments,
    PublishProgressCallback? onPublishProgress,
  }) async {
    File file = fileSystemEntity as File;

    // 使用配置类解析参数
    final config = PublishPgyerConfig.parse(environment, publishArguments);
    print(
        'config:\n${const JsonEncoder.withIndent('  ').convert(config.toJson())}');

    var tokenInfo = await getCOSToken(config, file.path);
    String uploadKey = await uploadApp(tokenInfo, file, onPublishProgress);
    if (uploadKey.isEmpty) {
      throw PublishError('UploadApp error');
    }
    // 重试次数设置为 0
    tryCount = 0;
    var buildResult = await getBuildInfo(config.apiKey, uploadKey);
    String buildKey = buildResult.data!['data']['buildKey'];
    return PublishResult(
      url: 'http://www.pgyer.com/$buildKey',
    );
  }

  /// 获取上传 Token 信息
  ///
  /// 构建包含所有 pgyer API 参数的请求数据
  /// 参考文档: https://www.pgyer.com/doc/view/api#fastUploadApp
  ///
  /// [config] 配置信息，包含所有 pgyer API 参数
  /// [filePath] 文件路径，用于确定 buildType
  Future<Response> getCOSToken(
    PublishPgyerConfig config,
    String filePath,
  ) async {
    Map<String, dynamic> formDataMap = {
      '_api_key': config.apiKey,
      'buildType': filePath.split('.').last,
    };

    // 添加所有可选参数，只在有值时才添加
    _addOptionalParameter(formDataMap, 'oversea', config.oversea);
    _addOptionalParameter(
        formDataMap, 'buildInstallType', config.buildInstallType);
    _addOptionalParameter(formDataMap, 'buildPassword', config.buildPassword);
    _addOptionalParameter(
        formDataMap, 'buildDescription', config.buildDescription);
    _addOptionalParameter(
        formDataMap, 'buildUpdateDescription', config.buildUpdateDescription);
    _addOptionalParameter(
        formDataMap, 'buildInstallDate', config.buildInstallDate);
    _addOptionalParameter(
        formDataMap, 'buildInstallStartDate', config.buildInstallStartDate);
    _addOptionalParameter(
        formDataMap, 'buildInstallEndDate', config.buildInstallEndDate);
    _addOptionalParameter(
        formDataMap, 'buildChannelShortcut', config.buildChannelShortcut);

    FormData formData = FormData.fromMap(formDataMap);
    try {
      Response response = await _dio.post(
        'https://www.pgyer.com/apiv2/app/getCOSToken',
        data: formData,
      );
      if (response.data['code'] != 0) {
        throw PublishError('getCOSToken error: ${response.data}');
      }
      return response;
    } catch (e) {
      throw PublishError(e.toString());
    }
  }

  /// 添加可选参数到表单数据中
  ///
  /// 只在参数不为空且不为空字符串时才添加
  ///
  /// [formDataMap] 表单数据映射
  /// [key] 参数键名
  /// [value] 参数值
  void _addOptionalParameter(
      Map<String, dynamic> formDataMap, String key, dynamic value) {
    if (value != null) {
      if (value is String && value.isNotEmpty) {
        formDataMap[key] = value;
      } else if (value is! String) {
        formDataMap[key] = value;
      }
    }
  }

  /// 上传应用
  /// [tokenInfo] token信息
  /// [file] 文件
  /// [onPublishProgress] 进度回调
  Future<String> uploadApp(
    Response tokenInfo,
    File file,
    PublishProgressCallback? onPublishProgress,
  ) async {
    var tokenData = tokenInfo.data['data'];
    String endpoint = tokenData['endpoint'];
    String key = tokenData['key'];
    var params = tokenData['params'];
    FormData formData = FormData.fromMap({
      'key': key,
      'signature': params['signature'],
      'x-cos-security-token': params['x-cos-security-token'],
      'x-cos-meta-file-name': file.path.split('/').last,
      'file': await MultipartFile.fromFile(file.path),
    });

    try {
      Response response = await _dio.post(
        endpoint,
        data: formData,
        onSendProgress: (int sent, int total) {
          if (onPublishProgress != null) {
            onPublishProgress(sent, total);
          }
        },
      );
      if (response.statusCode == 204) {
        // 上传成功，准备轮询结果
        return key;
      }
    } catch (e) {
      throw PublishError(e.toString());
    }
    return '';
  }

  /// 获取应用发布构建信息
  /// [apiKey] apiKey
  /// [uploadKey] uploadKey
  Future<Response> getBuildInfo(String apiKey, String uploadKey) async {
    if (tryCount > maxTryCount) {
      throw PublishError('getBuildInfo error :Too many retries');
    }
    await Future.delayed(const Duration(seconds: 3));
    try {
      Response response = await _dio.get(
        'https://www.pgyer.com/apiv2/app/buildInfo',
        queryParameters: {
          '_api_key': apiKey,
          'buildKey': uploadKey,
        },
      );
      int code = response.data['code'];
      if (code == 1247) {
        tryCount++;
        print('应用发布信息获取中，请稍等 $tryCount');
        return await getBuildInfo(apiKey, uploadKey);
      } else if (code != 0) {
        throw PublishError('getBuildInfo error: ${response.data}');
      }
      return response;
    } catch (e) {
      throw PublishError(e.toString());
    }
  }
}
