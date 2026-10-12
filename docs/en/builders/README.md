# Builders

English | [简体中文](../../zh-Hans/builders/README.md)

Builders select a project's build tool, run the build command, and locate generated artifacts. Packagers consume build results; the two have distinct responsibilities.

## Current Builders

| Builder              | Platforms or targets                                  | CLI integration                                            | Documentation                             |
| -------------------- | ----------------------------------------------------- | ---------------------------------------------------------- | ----------------------------------------- |
| Gradle Android       | Android APK, AAB                                      | Connected through `package` and the package action         | [Gradle](gradle.md)                       |
| Gradle Multiplatform | Android, desktop, iOS framework                       | No top-level CLI integration                               | [Gradle](gradle.md#multiplatform-builder) |
| Xcode                | iOS IPA, macOS `.app`                                 | Connected through the package action and `package`         | [Xcode](xcode.md)                         |
| .NET                 | Windows `dotnet publish` directory                    | Connected through the package action and `package`         | [.NET](dotnet.md)                         |
| Flutter              | Android, iOS, macOS, Windows, Linux, Web, OpenHarmony | Connected to `build`, `package`, and the package action    | [Flutter Builder](flutter.md)             |
| Custom               | User-defined commands and artifact rules              | No top-level CLI integration                               | [Custom Builder](custom.md)               |

“Implemented” and “connected to the CLI” are different states. A builder without a top-level entry point cannot be selected through Fastforge commands.

## Current Routing

`fastforge package` and the `fastforge/package` action select a build path based on the presence of `pubspec.yaml` in the project root and the platform (given with `--platform` or inferred from the targets):

1. No `pubspec.yaml`, and the platform is `macos` or `ios`: select Xcode Builder.
2. No `pubspec.yaml`, and the platform is `android`: select Gradle Builder.
3. No `pubspec.yaml`, and the platform is `windows`: select .NET Builder.
4. No `pubspec.yaml`, and any other platform: fail, because those platforms require a Flutter project.
5. `pubspec.yaml` present: select Flutter Builder.

`fastforge build` does not route; it always uses Flutter Builder.

The current detection rules are intentionally simple. Run commands from the actual project root to avoid selecting the wrong builder.

## Choosing an Entry Point

- Flutter projects: use `fastforge package` for distributable formats on every platform, or `fastforge build` for raw artifacts.
- Native Android Gradle projects: use `fastforge package` or the workflow package action.
- Native iOS / macOS Xcode projects: use the workflow package action and pass project arguments through `build-args`.
- Native Windows .NET projects: use the workflow package action, or `fastforge package` with `--flutter-build-args project=...`; see [.NET Builder](dotnet.md).
- Builders without CLI integration: continue to use the project's own build commands; do not assume the Fastforge CLI supports them yet.

See [Building](../building.md) for build commands and the result structure.
