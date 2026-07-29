class CheckVersionResult {
  CheckVersionResult({
    this.latestVersion,
    this.currentVersion,
  });
  final String? latestVersion;
  final String? currentVersion;

  /// Whether a new version is available.
  bool get isNewVersionAvailable =>
      currentVersion != null &&
      latestVersion != null &&
      currentVersion!.compareTo(latestVersion!) < 0;
}
