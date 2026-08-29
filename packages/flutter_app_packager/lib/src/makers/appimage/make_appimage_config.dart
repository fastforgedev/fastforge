// ignore_for_file: flutter_style_todo,todo

import 'dart:io';

import 'package:flutter_app_packager/src/api/app_package_maker.dart';

class AppImageAction {
  AppImageAction({
    required this.label,
    required this.name,
    required this.arguments,
  });
  factory AppImageAction.fromJson(Map<String, dynamic> map) {
    return AppImageAction(
      label: map['label'] as String,
      name: map['name'] as String,
      arguments: (map['arguments'] as List<dynamic>).cast<String>(),
    );
  }
  String label;
  String name;
  List<String> arguments;

  Map<String, dynamic> toJson() {
    return {
      'label': label,
      'name': name,
      'arguments': arguments,
    };
  }
}

class MakeAppImageConfig extends MakeConfig {
  MakeAppImageConfig(
      {required this.displayName,
      required this.icon,
      this.keywords = const [],
      this.categories = const [],
      this.actions = const [],
      this.include = const [],
      this.startupNotify = true,
      this.genericName = 'A Flutter Application',
      this.supportedMimeType,
      this.metainfo,
      this.runtimeFile,
      this.excludeUsrLibs});
  factory MakeAppImageConfig.fromJson(Map<String, dynamic> map) {
    return MakeAppImageConfig(
        displayName: map['display_name'] as String,
        icon: map['icon'] as String,
        metainfo: map['metainfo'] as String?,
        include: (map['include'] as List<dynamic>? ?? []).cast<String>(),
        keywords: (map['keywords'] as List<dynamic>? ?? []).cast<String>(),
        categories: (map['categories'] as List<dynamic>? ?? []).cast<String>(),
        startupNotify: map['startup_notify'] as bool? ?? false,
        genericName: map['generic_name'] as String? ?? 'A Flutter Application',
        actions: (map['actions'] as List? ?? [])
            .map(
              (e) => AppImageAction.fromJson(
                (Map.castFrom<dynamic, dynamic, String, dynamic>(e)),
              ),
            )
            .toList(),
        supportedMimeType: map['supported_mime_type'] != null
            ? List.castFrom<dynamic, String>(map['supported_mime_type'])
            : null,
        runtimeFile:
            map['runtime_file'] != null ? map['runtime_file'] as String : null,
        excludeUsrLibs: map['exclude_usr_libs'] != null
            ? List.castFrom<dynamic, String>(map['exclude_usr_libs'])
            : null);
  }

  final String icon;
  final String? metainfo;
  final List<String> keywords;
  final List<String> categories;
  final List<AppImageAction> actions;
  final bool startupNotify;
  final String genericName;
  final String displayName;
  final List<String> include;
  List<String>? supportedMimeType;
  final String? runtimeFile;
  final List<String>? excludeUsrLibs;

  String get desktopFileContent {
    final fields = {
      'Name': displayName,
      'GenericName': genericName,
      'Exec': 'LD_LIBRARY_PATH=usr/lib $appName %u',
      'Icon': appName,
      'Type': 'Application',
      'StartupNotify': startupNotify ? 'true' : 'false',
      if (supportedMimeType != null && supportedMimeType!.isNotEmpty)
        'MimeType': '${supportedMimeType!.join(';')};',
      if (categories.isNotEmpty) 'Categories': categories.join(';'),
      if (keywords.isNotEmpty) 'Keywords': keywords.join(';'),
      if (this.actions.isNotEmpty)
        'Actions': this.actions.map((e) => e.label).join(';'),
    }.entries.map((e) => '${e.key}=${e.value}').join('\n');

    final actions = this.actions.map((action) {
      final fields = {
        'Name': action.name,
        'Exec':
            'LD_LIBRARY_PATH=usr/lib $appName ${action.arguments.join(' ')} %u',
      };
      return '[Desktop Action ${action.label}]\n${fields.entries.map((e) => '${e.key}=${e.value}').join('\n')}';
    }).join('\n\n');

    return '[Desktop Entry]\n$fields\n\n$actions';
  }

  String get appRunContent {
    return '''
#!/bin/bash

cd "\$(dirname "\$0")"
export LD_LIBRARY_PATH=usr/lib
exec ./$appName
''';
  }
}

class MakeAppImageConfigLoader extends DefaultMakeConfigLoader {
  @override
  MakeConfig load(
    Map<String, dynamic>? arguments,
    Directory outputDirectory, {
    required Directory buildOutputDirectory,
    required List<File> buildOutputFiles,
  }) {
    final baseMakeConfig = super.load(
      arguments,
      outputDirectory,
      buildOutputDirectory: buildOutputDirectory,
      buildOutputFiles: buildOutputFiles,
    );
    final map = loadMakeConfigYaml(
      '$platform/packaging/$packageFormat/make_config.yaml',
    );
    return MakeAppImageConfig.fromJson(map).copyWith(baseMakeConfig);
  }
}
