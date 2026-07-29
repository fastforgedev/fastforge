import 'package:logging/logging.dart';

Logger logger = Logger('fastforge')
  ..onRecord.listen((record) {
    print(record.message);
  });
