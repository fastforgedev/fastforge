import 'dart:io';

import 'package:flutter_app_packager/src/api/app_package_maker.dart';
import 'package:path/path.dart' as p;

class MakeExeConfig extends MakeConfig {
  MakeExeConfig({
    this.scriptTemplate,
    required this.appId,
    this.executableName,
    this.displayName,
    this.publisherName,
    this.publisherUrl,
    this.createDesktopIcon,
    this.launchAtStartup,
    this.installDirName,
    this.setupIconFile,
    this.privilegesRequired,
    this.locales,
    this.architecturesAllowed,
    this.architecturesInstallIn64BitMode,
  });

  factory MakeExeConfig.fromJson(Map<String, dynamic> json) {
    List<String>? locales =
        json['locales'] != null ? List<String>.from(json['locales']) : null;
    if (locales == null || locales.isEmpty) locales = ['en'];

    // use absolute path
    String iconfile = '';
    if (json['setup_icon_file'] != null) {
      String currentDirectory = Directory.current.path;
      iconfile = p.join(currentDirectory, json['setup_icon_file']);
    }

    MakeExeConfig makeExeConfig = MakeExeConfig(
      scriptTemplate: json['script_template'],
      appId: json['app_id'] ?? json['appId'],
      executableName: json['executable_name'],
      displayName: json['display_name'],
      publisherName: json['publisher_name'] ?? json['appPublisher'],
      publisherUrl: json['publisher_url'] ?? json['appPublisherUrl'],
      createDesktopIcon: json['create_desktop_icon'],
      launchAtStartup: json['launch_at_startup'],
      installDirName: json['install_dir_name'],
      setupIconFile: iconfile,
      privilegesRequired: json['privileges_required'],
      locales: locales,
      architecturesAllowed: json['architectures_allowed'],
      architecturesInstallIn64BitMode:
          json['architectures_install_in_64bit_mode'],
    );
    return makeExeConfig;
  }

  String? scriptTemplate;
  final String appId;
  String? executableName;
  String? displayName;
  String? publisherName;
  String? publisherUrl;
  bool? createDesktopIcon;
  bool? launchAtStartup;
  String? installDirName;
  String? setupIconFile;
  String? privilegesRequired;
  List<String>? locales;

  /// Space-separated list of architecture identifiers (or a boolean expression)
  /// specifying which architectures Setup is allowed to run on.
  /// Defaults to `x64compatible`.
  /// See: https://jrsoftware.org/ishelp/index.php?topic=setup_architecturesallowed
  String? architecturesAllowed;

  /// Space-separated list of architecture identifiers (or a boolean expression)
  /// specifying which architectures should enable 64-bit install mode.
  /// Defaults to `x64compatible`.
  /// See: https://jrsoftware.org/ishelp/index.php?topic=setup_architecturesinstallin64bitmode
  String? architecturesInstallIn64BitMode;

  String get defaultExecutableName {
    File executableFile = packagingDirectory
        .listSync()
        .where((e) => e.path.endsWith('.exe'))
        .map((e) => File(e.path))
        .first;
    return p.basename(executableFile.path);
  }

  String get defaultInstallDirName => '{autopf64}\\$appName';

  String get sourceDir => p.basename(packagingDirectory.path);
  String get outputBaseFileName =>
      p.basename(outputFile.path).replaceAll('.exe', '');

  @override
  Map<String, dynamic> toJson() {
    return {
      'script_template': scriptTemplate,
      'app_id': appId,
      'app_name': appName,
      'app_version': appVersion.toString(),
      'executable_name': executableName,
      'display_name': displayName,
      'publisher_name': publisherName,
      'publisher_url': publisherUrl,
      'create_desktop_icon': createDesktopIcon,
      'launch_at_startup': launchAtStartup,
      'install_dir_name': installDirName,
      'setup_icon_file': setupIconFile,
      'privileges_required': privilegesRequired,
      'locales': locales,
      'architectures_allowed': architecturesAllowed,
      'architectures_install_in_64bit_mode': architecturesInstallIn64BitMode,
    }..removeWhere((key, value) => value == null);
  }
}

class MakeExeConfigLoader extends DefaultMakeConfigLoader {
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
    return MakeExeConfig.fromJson(map).copyWith(baseMakeConfig)
      ..isInstaller = true;
  }
}
