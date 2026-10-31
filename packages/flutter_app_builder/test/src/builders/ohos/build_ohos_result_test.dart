import 'dart:io';

import 'package:flutter_app_builder/src/build_config.dart';
import 'package:flutter_app_builder/src/builders/ohos/build_ohos_result.dart';
import 'package:path/path.dart' as p;
import 'package:test/test.dart';

void main() {
  late Directory workingDirectory;
  late Directory originalWorkingDirectory;

  setUp(() {
    originalWorkingDirectory = Directory.current;
    workingDirectory = Directory.systemTemp.createTempSync('ohos_result_test.');
    Directory.current = workingDirectory;
  });

  tearDown(() {
    Directory.current = originalWorkingDirectory;
    workingDirectory.deleteSync(recursive: true);
  });

  for (final target in ['hap', 'app']) {
    for (final flavor in [null, 'dev']) {
      group('$target result (${flavor ?? 'default'})', () {
        late BuildConfig config;
        late BuildOhosResultResolver resolver;
        late Directory outputDirectory;
        late String filename;

        setUp(() {
          config = BuildConfig(
            arguments: {if (flavor != null) 'flavor': flavor},
          );
          resolver = BuildOhosResultResolver(target);
          outputDirectory = BuildOhosResult(target, config).outputDirectory
            ..createSync(recursive: true);
          filename = '${outputDirectory.path}/entry-${flavor ?? 'default'}';
        });

        test('resolves unsigned output when no signed output exists', () {
          final unsigned = File('$filename-unsigned.$target')
            ..writeAsStringSync('unsigned');

          expect(
            resolver
                .resolve(config)
                .outputFiles
                .map((file) => p.normalize(file.path)),
            [p.normalize(unsigned.path)],
          );
        });

        test('prefers signed output when both outputs exist', () {
          File('$filename-unsigned.$target').writeAsStringSync('unsigned');
          final signed = File('$filename-signed.$target')
            ..writeAsStringSync('signed');

          expect(
            resolver
                .resolve(config)
                .outputFiles
                .map((file) => p.normalize(file.path)),
            [p.normalize(signed.path)],
          );
        });

        test('no-codesign ignores an existing signed output', () {
          final unsigned = File('$filename-unsigned.$target')
            ..writeAsStringSync('unsigned');
          File('$filename-signed.$target').writeAsStringSync('signed');
          config.arguments['no-codesign'] = true;

          expect(
            resolver
                .resolve(config)
                .outputFiles
                .map((file) => p.normalize(file.path)),
            [p.normalize(unsigned.path)],
          );
        });

        test('ignores other flavors and package formats', () {
          File('${outputDirectory.path}/entry-other-signed.$target')
              .writeAsStringSync('other flavor');
          File('$filename-signed.zip').writeAsStringSync('other format');

          expect(resolver.resolve(config).outputFiles, isEmpty);
        });
      });
    }
  }
}
