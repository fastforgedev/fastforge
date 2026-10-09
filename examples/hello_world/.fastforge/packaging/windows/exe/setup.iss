[Setup]
AppId=${APP_ID}
AppName=${APP_DISPLAY_NAME}
AppVersion=${BUILD_NAME}
AppPublisher=LiJianying
AppPublisherURL=${APP_HOMEPAGE}
DefaultDirName=${INSTALL_DIR}
DisableProgramGroupPage=yes
OutputBaseFilename=${OUTPUT_BASE_FILENAME}
Compression=lzma
SolidCompression=yes
WizardStyle=modern
PrivilegesRequired=none
ArchitecturesAllowed=${PACKAGE_ARCH}
ArchitecturesInstallIn64BitMode=${PACKAGE_ARCH}

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: checkedonce
Name: "launchAtStartup"; Description: "Start ${APP_DISPLAY_NAME} at sign-in"; GroupDescription: "{cm:AdditionalIcons}"; Flags: checkedonce

[Files]
Source: "${PACKAGING_DIRECTORY}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{autoprograms}\${APP_DISPLAY_NAME}"; Filename: "{app}\${EXECUTABLE_NAME}"
Name: "{autodesktop}\${APP_DISPLAY_NAME}"; Filename: "{app}\${EXECUTABLE_NAME}"; Tasks: desktopicon
Name: "{userstartup}\${APP_DISPLAY_NAME}"; Filename: "{app}\${EXECUTABLE_NAME}"; WorkingDir: "{app}"; Tasks: launchAtStartup

[Run]
Filename: "{app}\${EXECUTABLE_NAME}"; Description: "{cm:LaunchProgram,${APP_DISPLAY_NAME}}"; Flags: nowait postinstall skipifsilent
