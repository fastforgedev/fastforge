import 'package:fastforge/src/release_job.dart';

class Release {
  Release({
    this.variables,
    required this.name,
    required this.jobs,
  });

  factory Release.fromJson(Map<String, dynamic> json) {
    Map<String, String> variables = {};
    if (json.containsKey('variables') && json['variables'] != null) {
      variables = Map<String, String>.from(json['variables']);
    }
    List<ReleaseJob> jobs = (json['jobs'] as List? ?? [])
        .map((item) => ReleaseJob.fromJson(item))
        .toList();
    return Release(
      variables: variables,
      name: json['name'],
      jobs: jobs,
    );
  }

  final Map<String, String>? variables;
  final String name;
  final List<ReleaseJob> jobs;

  Map<String, dynamic> toJson() {
    return {
      'variables': variables,
      'name': name,
      'jobs': jobs.map((e) => e.toJson()).toList(),
    }..removeWhere((key, value) => value == null);
  }
}
