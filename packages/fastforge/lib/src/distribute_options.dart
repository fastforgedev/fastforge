import 'dart:io';

import 'package:fastforge/src/release.dart';

class DistributeOptions {
  DistributeOptions({
    required this.output,
    this.variables,
    this.artifactName,
    required this.releases,
  });

  factory DistributeOptions.fromJson(Map<String, dynamic> json) {
    Map<String, String> variables = {};
    if (json.containsKey('variables') && json['variables'] != null) {
      variables = Map<String, String>.from(json['variables']);
      // 兼容老版本
    } else if (json.containsKey('env') && json['env'] != null) {
      variables = Map<String, String>.from(json['env']);
    }
    List<Release> releases = ((json['releases'] ?? []) as List)
        .map((item) => Release.fromJson(item))
        .toList();
    return DistributeOptions(
      output: json['output'],
      variables: variables,
      artifactName: json['artifact_name'],
      releases: releases,
    );
  }

  final String output;
  final Map<String, String>? variables;
  final String? artifactName;
  final List<Release> releases;

  Directory get outputDirectory => Directory(output);

  Map<String, dynamic> toJson() {
    return {
      'output': output,
      'variables': variables,
      'artifact_name': artifactName,
      'releases': releases.map((e) => e.toJson()).toList(),
    }..removeWhere((key, value) => value == null);
  }
}
