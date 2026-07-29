import 'package:ansicolor/ansicolor.dart';

AnsiPen _ansiPen = AnsiPen();

const ansiResetForeground = '${ansiEscape}39m';

extension StringExt on String {
  String _applyColor(int color, {bool bg = false, bool bold = false}) {
    String appliedColor = (_ansiPen..xterm(color, bg: bg))(this);
    if (bold) {
      return '${ansiEscape}1m$appliedColor$ansiDefault';
    }
    return appliedColor;
  }

  String black({bool bg = false, bool bold = false}) {
    return _applyColor(0, bg: bg, bold: bold);
  }

  String red({bool bg = false, bool bold = false}) {
    return _applyColor(1, bg: bg, bold: bold);
  }

  String green({bool bg = false, bool bold = false}) {
    return _applyColor(2, bg: bg, bold: bold);
  }

  String yellow({bool bg = false, bool bold = false}) {
    return _applyColor(3, bg: bg, bold: bold);
  }

  String blue({bool bg = false, bool bold = false}) {
    return _applyColor(4, bg: bg, bold: bold);
  }

  String magenta({bool bg = false, bool bold = false}) {
    return _applyColor(5, bg: bg, bold: bold);
  }

  String cyan({bool bg = false, bool bold = false}) {
    return _applyColor(6, bg: bg, bold: bold);
  }

  String white({bool bg = false, bool bold = false}) {
    return _applyColor(7, bg: bg, bold: bold);
  }

  String brightBlack({bool bg = false, bool bold = false}) {
    return _applyColor(8, bg: bg, bold: bold);
  }

  String brightRed({bool bg = false, bool bold = false}) {
    return _applyColor(9, bg: bg, bold: bold);
  }

  String brightGreen({bool bg = false, bool bold = false}) {
    return _applyColor(10, bg: bg, bold: bold);
  }

  String brightYellow({bool bg = false, bool bold = false}) {
    return _applyColor(11, bg: bg, bold: bold);
  }

  String brightBlue({bool bg = false, bool bold = false}) {
    return _applyColor(12, bg: bg, bold: bold);
  }

  String brightMagenta({bool bg = false, bool bold = false}) {
    return _applyColor(13, bg: bg, bold: bold);
  }

  String brightCyan({bool bg = false, bool bold = false}) {
    return _applyColor(14, bg: bg, bold: bold);
  }

  String brightWhite({bool bg = false, bool bold = false}) {
    return _applyColor(15, bg: bg, bold: bold);
  }
}
