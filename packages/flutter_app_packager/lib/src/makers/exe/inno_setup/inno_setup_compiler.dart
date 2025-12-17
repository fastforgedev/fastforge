import 'dart:io';

import 'package:flutter_app_packager/src/makers/exe/inno_setup/inno_setup_script.dart';
import 'package:path/path.dart' as p;
import 'package:shell_executor/shell_executor.dart';

class InnoSetupCompiler {
  Future<bool> compile(InnoSetupScript script) async {
    File file = await script.createFile();

    ProcessResult processResult;

    // First, try the default installation path
    Directory innoSetupDirectory =
        Directory('C:\\Program Files (x86)\\Inno Setup 6');

    if (innoSetupDirectory.existsSync()) {
      // Use ISCC from the default installation directory
      processResult = await $(
        p.join(innoSetupDirectory.path, 'ISCC.exe'),
        [file.path],
      );
    } else {
      // Fall back to PATH
      try {
        processResult = await $('ISCC', [file.path]);
      } on ProcessException {
        throw Exception(
            '\'Inno Setup 6\' was not installed or ISCC is not in PATH.');
      }
    }

    if (processResult.exitCode != 0) {
      return false;
    }

    file.deleteSync(recursive: true);
    return true;
  }
}
