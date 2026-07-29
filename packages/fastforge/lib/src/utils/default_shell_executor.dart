import 'dart:convert';
import 'dart:io';

import 'package:charset/charset.dart';
import 'package:fastforge/src/extensions/string.dart';
import 'package:fastforge/src/utils/logger.dart';
import 'package:shell_executor/shell_executor.dart';

/// Convert bytes to string (UTF-8 or detected charset)
String convertToString(List<int> bytes) {
  if (Platform.isWindows) {
    final charset = Charset.detect(bytes);
    if (charset != null) {
      return charset.decode(bytes);
    }
  }
  return utf8.decode(bytes, allowMalformed: true);
}

class DefaultShellExecutor extends ShellExecutor {
  @override
  Future<ProcessResult> exec(
    String executable,
    List<String> arguments, {
    String? workingDirectory,
    Map<String, String>? environment,
  }) async {
    final Process process = await Process.start(
      executable,
      arguments,
      workingDirectory: workingDirectory,
      environment: environment,
      runInShell: true,
    );

    logger.info('\$ $executable ${arguments.join(' ')}'.brightBlack());

    String? stdoutStr;
    String? stderrStr;

    process.stdout.listen((data) {
      String msg = convertToString(data);
      stdoutStr = '${stdoutStr ?? ''}$msg';
      stdout.write(msg.brightBlack());
    });
    process.stderr.listen((data) {
      String msg = convertToString(data);
      stderrStr = '${stderrStr ?? ''}$msg';
      stderr.write(msg.brightRed());
    });
    int exitCode = await process.exitCode;
    return ProcessResult(process.pid, exitCode, stdoutStr, stderrStr);
  }
}
