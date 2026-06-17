import 'dart:io';

import 'package:flutter_app_packager/src/api/app_package_maker.dart';
import 'package:flutter_app_packager/src/makers/appimage/make_appimage_config.dart';
import 'package:path/path.dart' as path;
import 'package:shell_executor/shell_executor.dart';

class AppPackageMakerAppImage extends AppPackageMaker {
  @override
  String get name => 'appimage';
  @override
  String get platform => 'linux';
  @override
  bool get isSupportedOnCurrentPlatform => Platform.isLinux;
  @override
  String get packageFormat => 'appimage';

  @override
  MakeConfigLoader get configLoader {
    return MakeAppImageConfigLoader()
      ..platform = platform
      ..packageFormat = packageFormat;
  }

  Future<Set<String>> _getSharedDependencies(String so) {
    return $('ldd', ['-d', so]).then((value) {
      if (value.exitCode != 0) {
        throw MakeError(value.stderr as String);
      }
      return value.stdout as String;
    }).then(
      (lines) {
        final soDeps = lines
            .split('\n')
            .where(
              (line) => line.contains('=>') && line.trim().startsWith('lib'),
            )

            /// converts this:
            ///  libkeybinder-3.0.so.0 => /lib64/libkeybinder-3.0.so.0 (0x00007f6513811000)
            /// to this:
            ///  /lib64/libkeybinder-3.0.so.0
            .map((line) => line.split(' => ')[1].trim().split(' ').first.trim())
            .toList()
          ..sort();

        return soDeps.toSet();
      },
    );
  }

  @override
  Future<MakeResult> make(MakeConfig config) {
    return _make(
      config.buildOutputDirectory,
      outputDirectory: config.outputDirectory,
      makeConfig: config as MakeAppImageConfig,
    );
  }

  Future<MakeResult> _make(
    Directory appDirectory, {
    required Directory outputDirectory,
    required MakeAppImageConfig makeConfig,
  }) async {
    try {
      await $('cp', [
        '-r',
        appDirectory.path,
        path.join(
          makeConfig.packagingDirectory.path,
          '${makeConfig.appName}.AppDir',
        ),
      ]).then((value) {
        if (value.exitCode != 0) {
          throw MakeError(value.stderr as String);
        }
      });

      final desktopFile = File(
        path.join(
          makeConfig.packagingDirectory.path,
          '${makeConfig.appName}.AppDir',
          '${makeConfig.appName}.desktop',
        ),
      )..createSync(recursive: true);

      await desktopFile.writeAsString(makeConfig.desktopFileContent);

      final appRunFile = File(
        path.join(
          makeConfig.packagingDirectory.path,
          '${makeConfig.appName}.AppDir',
          'AppRun',
        ),
      )..createSync(recursive: true);

      await appRunFile.writeAsString(makeConfig.appRunContent);

      await $('chmod', ['+x', appRunFile.path]).then((value) {
        if (value.exitCode != 0) {
          throw MakeError(value.stderr as String);
        }
      });

      final iconFile = File(makeConfig.icon);
      if (!iconFile.existsSync()) {
        throw MakeError("icon ${makeConfig.icon} path doesn't exist");
      }

      await iconFile.copy(
        path.join(
          makeConfig.packagingDirectory.path,
          '${makeConfig.appName}.AppDir',
          '${makeConfig.appName}${path.extension(makeConfig.icon)}',
        ),
      );

      final icon256x256 = path.join(
        makeConfig.packagingDirectory.path,
        '${makeConfig.appName}.AppDir/usr/share/icons/hicolor/256x256/apps',
      );
      final icon128x128 = path.join(
        makeConfig.packagingDirectory.path,
        '${makeConfig.appName}.AppDir/usr/share/icons/hicolor/128x128/apps',
      );

      await $('mkdir', [
        '-p',
        icon128x128,
        icon256x256,
      ]).then((value) {
        if (value.exitCode != 0) {
          throw MakeError(value.stderr as String);
        }
      });

      await iconFile.copy(
        path.join(
          icon128x128,
          '${makeConfig.appName}${path.extension(makeConfig.icon)}',
        ),
      );

      await iconFile.copy(
        path.join(
          icon256x256,
          '${makeConfig.appName}${path.extension(makeConfig.icon)}',
        ),
      );

      if (makeConfig.metainfo != null) {
        final metainfoDir = path.join(
          makeConfig.packagingDirectory.path,
          '${makeConfig.appName}.AppDir/usr/share/metainfo',
        );
        await $('mkdir', [
          '-p',
          metainfoDir,
        ]).then((value) {
          if (value.exitCode != 0) {
            throw MakeError(value.stderr as String);
          }
        });
        final metainfoPath =
            path.join(Directory.current.path, makeConfig.metainfo!);
        final metainfoFile = File(metainfoPath);
        if (!metainfoFile.existsSync()) {
          throw MakeError("Metainfo $metainfoPath path doesn't exist");
        }
        await metainfoFile.copy(
          path.join(
            metainfoDir,
            makeConfig.appBinaryName + path.extension(makeConfig.metainfo!, 2),
          ),
        );
      }

      final defaultSharedObjects = [
        'libapp.so',
        'libflutter_linux_gtk.so',
        'libgtk-3.so.0',
      ];

      final appSOLibs = Directory(
        path.join(
          makeConfig.packagingDirectory.path,
          '${makeConfig.appName}.AppDir/lib',
        ),
      ).listSync().where(
            (e) =>
                !defaultSharedObjects.contains(path.basename(e.path)) &&
                !FileSystemEntity.isDirectorySync(e.path),
          );

      await $('mkdir', [
        '-p',
        path.join(
          makeConfig.packagingDirectory.path,
          '${makeConfig.appName}.AppDir/usr/lib',
        ),
      ]).then((value) {
        if (value.exitCode != 0) {
          throw MakeError(value.stderr as String);
        }
      });

      final libFlutterGtkDeps = await _getSharedDependencies(
        path.join(
          makeConfig.packagingDirectory.path,
          '${makeConfig.appName}.AppDir/lib/libflutter_linux_gtk.so',
        ),
      );

      await Future.wait(
        appSOLibs.map((so) async {
          final referencedSharedLibs =
              await _getSharedDependencies(so.path).then(
            (d) => d.difference(libFlutterGtkDeps)
              ..removeWhere(
                (lib) => lib.contains('libflutter_linux_gtk.so'),
              ),
          );

          if (referencedSharedLibs.isEmpty) return;

          await $(
            'cp',
            [
              ...referencedSharedLibs,
              path.join(
                makeConfig.packagingDirectory.path,
                '${makeConfig.appName}.AppDir/usr/lib',
              ),
            ],
          ).then((value) {
            if (value.exitCode != 0) {
              throw MakeError(value.stderr as String);
            }
          });
        }),
      );

      await Future.wait(
        makeConfig.include.map((so) async {
          final file = await $('locate', [so]).then((value) {
            if (value.exitCode != 0) {
              throw MakeError(value.stderr as String);
            }
            return value.stdout as String;
          }).then((out) {
            final paths = out
                .split('\n')
                .where((p) => p.isNotEmpty && !p.contains('/Trash'))
                .toList();
            if (paths.isEmpty) {
              throw MakeError("Can't find specified shared object $so");
            }
            return File(paths.first.trim());
          });

          await file.copy(
            path.join(
              makeConfig.packagingDirectory.path,
              '${makeConfig.appName}.AppDir/usr/lib/',
              path.basename(file.path),
            ),
          );
        }),
      );

      var outputMakeConfig = MakeConfig().copyWith(makeConfig)
        ..packageFormat = 'AppImage';

      await $(
        'appimagetool',
        [
          '--no-appstream',
          path.join(
            makeConfig.packagingDirectory.path,
            '${makeConfig.appName}.AppDir',
          ),
          outputMakeConfig.outputFile.path,
        ],
        environment: {
          'ARCH': 'x86_64',
        },
      ).then((value) {
        if (value.exitCode != 0) {
          throw MakeError(value.stderr as String);
        }
      });

      makeConfig.packagingDirectory.deleteSync(recursive: true);
      return MakeResult(outputMakeConfig);
    } catch (e) {
      if (e is MakeError) rethrow;
      throw MakeError(e.toString());
    }
  }
}
