import 'dart:io';

import 'package:flutter_app_packager/src/api/app_package_maker.dart';

class MakeRPMConfig extends MakeLinuxPackageConfig {
  MakeRPMConfig({
    // Desktop file
    required this.displayName,
    this.packageName,
    this.startupNotify = true,
    this.actions,
    this.categories,
    this.genericName,
    this.icon,
    this.metainfo,
    this.keywords,
    this.supportedMimeType,

    // Spec file
    this.summary,
    this.group,
    this.vendor,
    this.packager,
    this.packagerEmail,
    this.license,
    this.url,
    this.buildArch,
    this.requires,
    this.buildRequires,
    this.description,
    this.prep,
    this.build,
    this.install,
    this.postun,
    List<String>? postinstallScripts,
    List<String>? postuninstallScripts,
    this.files,
    this.defattr,
    this.attr,
    this.changelog,
    this.specMacros,
  })  : _postinstallScripts = postinstallScripts ?? [],
        _postuninstallScripts = postuninstallScripts ?? [];

  factory MakeRPMConfig.fromJson(Map<String, dynamic> json) {
    return MakeRPMConfig(
      displayName: json['display_name'] as String,
      packageName: json['package_name'] as String?,
      icon: json['icon'] as String?,
      metainfo: json['metainfo'] as String?,
      genericName: json['generic_name'] as String?,
      startupNotify: json['startup_notify'] as bool?,
      keywords: (json['keywords'] as List<dynamic>?)?.cast<String>(),
      supportedMimeType:
          (json['supported_mime_type'] as List<dynamic>?)?.cast<String>(),
      actions: (json['actions'] as List<dynamic>?)?.cast<String>(),
      categories: (json['categories'] as List<dynamic>?)?.cast<String>(),
      summary: json['summary'] as String?,
      group: json['group'] as String?,
      vendor: json['vendor'] as String?,
      packager: json['packager'] as String?,
      packagerEmail: (json['packagerEmail'] ?? json['packager_email']) as String?,
      license: json['license'] as String?,
      url: json['url'] as String?,
      buildArch: json['build_arch'] as String? ?? _getArchitecture(),
      requires: (json['requires'] as List<dynamic>?)?.cast<String>(),
      buildRequires: (json['build_requires'] as List<dynamic>?)?.cast<String>(),
      description: json['description'] as String?,
      prep: json['prep'] as String?,
      build: json['build'] as String?,
      install: json['install'] as String?,
      postinstallScripts: json['postinstall_scripts'] != null
          ? List.castFrom<dynamic, String>(json['postinstall_scripts'])
          : null,
      postuninstallScripts: json['postuninstall_scripts'] != null
          ? List.castFrom<dynamic, String>(json['postuninstall_scripts'])
          : null,
      postun: json['postun'] as String?,
      files: json['files'] as String?,
      defattr: json['defattr'] as String?,
      attr: json['attr'] as String?,
      changelog: json['changelog'] as String?,
      specMacros: (json['spec_macros'] as List<dynamic>?)?.cast<String>(),
    );
  }

  String displayName;
  String? packageName;
  String? icon;
  String? metainfo;
  String? genericName;
  bool? startupNotify;
  List<String>? keywords;
  List<String>? supportedMimeType;
  List<String>? actions;
  List<String>? categories;
  List<String> _postinstallScripts;
  List<String> _postuninstallScripts;

  //RPM preamble Spec file fields
  String? summary;
  String? group;
  String? vendor;
  String? packager;
  String? packagerEmail;
  String? license;
  String? url;
  String? buildArch;
  List<String>? requires;
  List<String>? buildRequires;
  //RPM postamble Spec file fields
  String? description;
  String? prep;
  String? build;
  String? install;
  String? postun;
  String? files;
  String? defattr;
  String? attr;
  String? changelog;
  List<String>? specMacros;

  List<String> get postScripts => [
    'update-mime-database %{_datadir}/mime &> /dev/null || :',
    ..._postinstallScripts,
  ];

  List<String> get postunScripts => [
    'update-mime-database %{_datadir}/mime &> /dev/null || :',
    if (postun != null) postun!,
    ..._postuninstallScripts,
  ];

  @override
  Map<String, dynamic> toJson() {
    return {
      'SPEC': {
        'preamble': {
          'Name': packageName ?? appName,
          'Version': appVersion.toString(),
          'Release':
              "${appVersion.build.isNotEmpty ? appVersion.build.first : "1"}%{?dist}",
          'Summary': summary ?? pubspec.description,
          'Group': group,
          'Vendor': vendor,
          'Packager':
              packagerEmail != null ? '$packager <$packagerEmail>' : packager,
          'License': license,
          'URL': url,
          'Requires': requires?.join(', '),
          'BuildRequires': buildRequires?.join(', '),
          'BuildArch': buildArch ?? _getArchitecture(),
        }..removeWhere((key, value) => value == null),
        'body': {
          '%description': description ?? pubspec.description,
          '%install': [
            'cd "%{_topdir}/BUILD"',
            'mkdir -p %{buildroot}%{_bindir}',
            'mkdir -p %{buildroot}%{_datadir}/%{name}',
            'mkdir -p %{buildroot}%{_datadir}/applications',
            'mkdir -p %{buildroot}%{_datadir}/metainfo',
            'mkdir -p %{buildroot}%{_datadir}/pixmaps',
            'cp -r $appName/* %{buildroot}%{_datadir}/%{name}',
            'ln -s %{_datadir}/%{name}/$appBinaryName %{buildroot}%{_bindir}/%{name}',
            'cp -r $appBinaryName.desktop %{buildroot}%{_datadir}/applications/%{name}.desktop',
            'cp -r $appBinaryName.png %{buildroot}%{_datadir}/pixmaps/%{name}.png',
            'cp -r $appBinaryName*.xml %{buildroot}%{_datadir}/metainfo/%{name}.appdata.xml || :',
          ].join('\n'),
          '%post': postScripts.join('\n'),
          '%postun': postunScripts.join('\n'),
          '%files': [
            '%{_bindir}/%{name}',
            '%{_datadir}/%{name}',
            '%{_datadir}/applications/%{name}.desktop',
            '%{_datadir}/metainfo',
          ].join('\n'),
        }..removeWhere((key, value) => value == null),
        'inline-body': {
          '%defattr': '(-,root,root)',
          '%attr': '(4755, root, root) %{_datadir}/pixmaps/%{name}.png',
        },
      },
      'DESKTOP': {
        'Type': 'Application',
        'Name': displayName,
        'GenericName': genericName,
        'Icon': packageName ?? appName,
        'Exec': '${packageName ?? appName} %U',
        'Actions': actions != null && actions!.isNotEmpty
            ? '${actions!.join(';')};'
            : null,
        'MimeType': supportedMimeType != null && supportedMimeType!.isNotEmpty
            ? '${supportedMimeType!.join(';')};'
            : null,
        'Categories': categories != null && categories!.isNotEmpty
            ? '${categories!.join(';')};'
            : null,
        'Keywords': keywords != null && keywords!.isNotEmpty
            ? '${keywords!.join(';')};'
            : null,
        'StartupNotify': startupNotify,
      }..removeWhere((key, value) => value == null),
    };
  }

  Map<String, String> toFilesString() {
    final json = toJson();

    final preamble = (json['SPEC']['preamble'] as Map)
        .entries
        .map((e) => '${e.key}: ${e.value}')
        .join('\n');
    final body = (json['SPEC']['body'] as Map).entries.map(
      (e) {
        return '${e.key}\n${e.value}\n';
      },
    ).join('\n');
    final inlineBody = (json['SPEC']['inline-body'] as Map).entries.map(
      (e) {
        return '${e.key}${e.value}\n';
      },
    ).join('\n');

    final desktopFile = [
      '[Desktop Entry]',
      ...(json['DESKTOP'] as Map<String, dynamic>).entries.map(
            (e) => '${e.key}=${e.value}',
          ),
    ].join('\n');
    final macrosStr = specMacros != null && specMacros!.isNotEmpty
        ? '${specMacros!.join('\n')}\n\n'
        : '';
    final map = {
      'DESKTOP': desktopFile,
      'SPEC': '$macrosStr$preamble\n\n$body\n\n$inlineBody',
    };
    return Map.castFrom<String, String?, String, String>(map);
  }
}

class MakeRpmConfigLoader extends DefaultMakeConfigLoader {
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
    return MakeRPMConfig.fromJson(map).copyWith(baseMakeConfig);
  }
}

String _getArchitecture() {
  final result = Process.runSync('uname', ['-m']);
  if ('${result.stdout}'.trim() == 'aarch64') {
    return 'aarch64';
  } else {
    return 'x86_64';
  }
}
