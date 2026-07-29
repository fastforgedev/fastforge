import 'dart:convert';
import 'dart:io';

import 'package:dio/dio.dart';

class PubDevApi {
  static Future<String?> getLatestVersionFromPackage(String package) async {
    String pubHostedUrl = Platform.environment['PUB_HOSTED_URL'] ?? '';
    final pubSite = pubHostedUrl.isNotEmpty
        ? '$pubHostedUrl/api/packages/$package'
        : 'https://pub.dev/api/packages/$package';
    final uri = Uri.parse(pubSite);
    try {
      final response = await Dio().get(uri.toString());
      Map<String, dynamic> data = {};
      if (response.data is String) {
        data = json.decode(response.data);
      } else {
        data = response.data;
      }
      if (data['latest'] == null) return null;
      return data['latest']['version'] as String?;
    } catch (error) {
      rethrow;
    }
  }
}
