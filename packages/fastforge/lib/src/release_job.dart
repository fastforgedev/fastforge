class ReleaseJobPackage {
  ReleaseJobPackage({
    required this.platform,
    required this.target,
    this.channel,
    this.buildArgs,
    this.hooks,
  });

  factory ReleaseJobPackage.fromJson(Map<String, dynamic> json) {
    Map<String, dynamic>? hooks;
    if (json.containsKey('hooks') && json['hooks'] != null) {
      hooks = Map<String, dynamic>.from(json['hooks']);
    }
    return ReleaseJobPackage(
      platform: json['platform'],
      target: json['target'],
      channel: json['channel'],
      buildArgs: json['build_args'],
      hooks: hooks,
    );
  }

  final String platform;
  final String target;
  final String? channel;
  final Map<String, dynamic>? buildArgs;

  /// Package lifecycle hooks, e.g. `{ "pre": "...", "post": "..." }`
  final Map<String, dynamic>? hooks;

  Map<String, dynamic> toJson() {
    return {
      'platform': platform,
      'target': target,
      'channel': channel,
      'build_args': buildArgs,
      'hooks': hooks,
    }..removeWhere((key, value) => value == null);
  }
}

class ReleaseJobPublish {
  ReleaseJobPublish({
    required this.target,
    this.args,
  });

  factory ReleaseJobPublish.fromJson(Map<String, dynamic> json) {
    return ReleaseJobPublish(
      target: json['target'],
      args: json['args'],
    );
  }
  final String target;
  final Map<String, dynamic>? args;

  Map<String, dynamic> toJson() {
    return {
      'target': target,
      'args': args,
    }..removeWhere((key, value) => value == null);
  }
}

class ReleaseJob {
  ReleaseJob({
    this.variables,
    required this.name,
    required this.package,
    this.publish,
    this.publishTo,
  });

  factory ReleaseJob.fromJson(Map<String, dynamic> json) {
    Map<String, String> variables = {};
    if (json.containsKey('variables') && json['variables'] != null) {
      variables = Map<String, String>.from(json['variables']);
    }
    return ReleaseJob(
      variables: variables,
      name: json['name'],
      package: ReleaseJobPackage.fromJson(json['package']),
      publish: json['publish'] != null
          ? ReleaseJobPublish.fromJson(json['publish'])
          : null,
      publishTo: json['publish_to'],
    );
  }

  final Map<String, String>? variables;
  final String name;
  final ReleaseJobPackage package;
  final ReleaseJobPublish? publish;
  final String? publishTo;

  Map<String, dynamic> toJson() {
    return {
      'variables': variables,
      'name': name,
      'package': package.toJson(),
      'publish': publish?.toJson(),
      'publish_to': publishTo,
    }..removeWhere((key, value) => value == null);
  }
}
