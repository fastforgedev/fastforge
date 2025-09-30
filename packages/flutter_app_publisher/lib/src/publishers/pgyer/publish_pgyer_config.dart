import 'dart:io';

import 'package:flutter_app_publisher/src/api/app_package_publisher.dart';

const kEnvPgyerApiKey = 'PGYER_API_KEY';

/// Pgyer 发布配置类
///
/// 支持 pgyer.com API 的所有可选参数
/// 文档参考: https://www.pgyer.com/doc/view/api#fastUploadApp
class PublishPgyerConfig extends PublishConfig {
  PublishPgyerConfig({
    required this.apiKey,
    this.oversea,
    this.buildInstallType,
    this.buildPassword,
    this.buildDescription,
    this.buildUpdateDescription,
    this.buildInstallDate,
    this.buildInstallStartDate,
    this.buildInstallEndDate,
    this.buildChannelShortcut,
  });

  /// 从环境变量和发布参数中解析配置
  factory PublishPgyerConfig.parse(
    Map<String, String>? environment,
    Map<String, dynamic>? publishArguments,
  ) {
    String? apiKey = (environment ?? Platform.environment)[kEnvPgyerApiKey];
    if ((apiKey ?? '').isEmpty) {
      throw PublishError('Missing `$kEnvPgyerApiKey` environment variable.');
    }

    return PublishPgyerConfig(
      apiKey: apiKey!,
      oversea: _parseInt(publishArguments?['oversea']),
      buildInstallType: _parseInt(publishArguments?['install-type']),
      buildPassword: _parseString(publishArguments?['password']),
      buildDescription: _parseString(publishArguments?['description']),
      buildUpdateDescription:
          _parseString(publishArguments?['update-description']),
      buildInstallDate: _parseInt(publishArguments?['install-date']),
      buildInstallStartDate:
          _parseString(publishArguments?['install-start-date']),
      buildInstallEndDate: _parseString(publishArguments?['install-end-date']),
      buildChannelShortcut: _parseString(publishArguments?['channel-shortcut']),
    );
  }

  /// Pgyer API Key (必需)
  final String apiKey;

  /// 是否使用海外加速上传
  /// 1: 使用海外加速上传
  /// 2: 国内加速上传
  /// null: 根据 IP 自动判断
  final int? oversea;

  /// 应用安装方式
  /// 1: 公开安装 (默认)
  /// 2: 密码安装
  /// 3: 邀请安装
  final int? buildInstallType;

  /// 设置App安装密码
  /// 密码为空时默认公开安装
  final String? buildPassword;

  /// 应用介绍
  /// 如没有介绍请传空字符串，或不传
  final String? buildDescription;

  /// 版本更新描述
  /// 请传空字符串，或不传
  final String? buildUpdateDescription;

  /// 是否设置安装有效期
  /// 1: 设置有效时间
  /// 2: 长期有效
  /// 如果不填写不修改上一次的设置
  final int? buildInstallDate;

  /// 安装有效期开始时间
  /// 字符串型，如：2018-01-01
  final String? buildInstallStartDate;

  /// 安装有效期结束时间
  /// 字符串型，如：2018-12-31
  final String? buildInstallEndDate;

  /// 所需更新指定的渠道短链接
  /// 渠道短链接须为已创建成功的，并且只可指定一个渠道
  /// 字符串型，如：abcd
  final String? buildChannelShortcut;

  /// 解析整数值
  ///
  /// 支持字符串和数字类型的转换
  ///
  /// [value] 要解析的值
  /// 返回解析后的整数值，如果解析失败则返回 null
  static int? _parseInt(dynamic value) {
    if (value == null) return null;
    if (value is int) return value;
    if (value is String) {
      return int.tryParse(value);
    }
    return null;
  }

  /// 解析字符串值
  ///
  /// 支持将任意类型安全转换为字符串；空字符串将按原样保留
  static String? _parseString(dynamic value) {
    if (value == null) return null;
    if (value is String) return value;
    return value.toString();
  }

  Map<String, dynamic> toJson() {
    return {
      'oversea': oversea,
      'buildInstallType': buildInstallType,
      'buildPassword': buildPassword,
      'buildDescription': buildDescription,
      'buildUpdateDescription': buildUpdateDescription,
      'buildInstallDate': buildInstallDate,
      'buildInstallStartDate': buildInstallStartDate,
      'buildInstallEndDate': buildInstallEndDate,
      'buildChannelShortcut': buildChannelShortcut,
    }..removeWhere((key, value) => value == null);
  }
}
