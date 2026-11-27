# fastforge Windows installer
# Usage:
#   iwr https://fastforge.dev/install.ps1 | iex
#   $env:FASTFORGE_VERSION="0.7.0"; iwr https://fastforge.dev/install.ps1 | iex
#   $env:GITHUB_TOKEN="ghp_xxx"; iwr https://fastforge.dev/install.ps1 | iex

$ErrorActionPreference = 'Stop'

$Repo        = "fastforgedev/fastforge"
$BinaryName  = "fastforge"
$InstallDir  = if ($env:FASTFORGE_INSTALL_DIR) { $env:FASTFORGE_INSTALL_DIR } else { "$env:LOCALAPPDATA\fastforge\bin" }

# ── Helpers ───────────────────────────────────────────────────────────────────
function Write-Info    { param($msg) Write-Host "info  $msg" -ForegroundColor Cyan }
function Write-Success { param($msg) Write-Host "✓     $msg" -ForegroundColor Green }
function Write-Warn    { param($msg) Write-Host "warn  $msg" -ForegroundColor Yellow }
function Write-Fail    { param($msg) Write-Host "error $msg" -ForegroundColor Red; exit 1 }

# ── Detect architecture ───────────────────────────────────────────────────────
function Get-Arch {
  $arch = $env:PROCESSOR_ARCHITECTURE
  switch ($arch) {
    "AMD64" { return "x86_64" }
    "ARM64" { return "aarch64" }
    default { Write-Fail "Unsupported architecture: $arch" }
  }
}

# ── Map to Rust target triple ─────────────────────────────────────────────────
function Get-Target {
  param($arch)
  switch ($arch) {
    "x86_64"   { return "x86_64-pc-windows-msvc" }
    "aarch64"  { return "aarch64-pc-windows-msvc" }
    default    { Write-Fail "No prebuilt binary available for architecture: $arch" }
  }
}

# ── Build common API request headers ─────────────────────────────────────────
function Get-ApiHeaders {
  $headers = @{ "User-Agent" = "fastforge-installer" }
  if ($env:GITHUB_TOKEN) {
    $headers["Authorization"] = "Bearer $($env:GITHUB_TOKEN)"
  }
  return $headers
}

# ── Resolve version + download URL ───────────────────────────────────────────
function Resolve-Release {
  param($target)

  # ── Manual version: construct URL directly ────────────────────────────────
  if ($env:FASTFORGE_VERSION) {
    $version     = $env:FASTFORGE_VERSION
    $archiveName = "$BinaryName-$version-$target.zip"
    $downloadUrl = "https://github.com/$Repo/releases/download/v$version/$archiveName"
    Write-Info "Using specified version: $version"
    return @{ Version = $version; ArchiveName = $archiveName; DownloadUrl = $downloadUrl }
  }

  # ── Auto-detect: fetch releases list ──────────────────────────────────────
  Write-Info "Fetching latest release version..."

  try {
    $headers  = Get-ApiHeaders
    $releases = Invoke-RestMethod `
      -Uri "https://api.github.com/repos/$Repo/releases?per_page=100" `
      -Headers $headers
  } catch {
    Write-Fail "Failed to fetch releases from GitHub: $_`nSet `$env:FASTFORGE_VERSION to specify a version manually."
  }

  # Pick the newest published (non-draft, non-prerelease) release that ships
  # a binary for this target (releases from the Dart era carry no binaries).
  $pattern = "^$([regex]::Escape($BinaryName))-(.+)-$([regex]::Escape($target))\.zip$"
  $asset = $releases |
    Where-Object { -not $_.draft -and -not $_.prerelease } |
    ForEach-Object { $_.assets } |
    Where-Object { $_.name -match $pattern } |
    Select-Object -First 1

  if (-not $asset) {
    Write-Fail "No published release provides a prebuilt binary for '$target'.`nSet `$env:FASTFORGE_VERSION to specify a version manually."
  }

  $archiveName = $asset.name
  $version     = [regex]::Match($archiveName, $pattern).Groups[1].Value
  if (-not $version) { Write-Fail "Failed to parse version from asset name '$archiveName'." }

  Write-Info "Latest version: $version"

  $downloadUrl = $asset.browser_download_url
  return @{ Version = $version; ArchiveName = $archiveName; DownloadUrl = $downloadUrl }
}

# ── Check if binary is already on PATH ───────────────────────────────────────
function Get-InstalledVersion {
  try {
    $v = & fastforge --version 2>$null
    return $v
  } catch {
    return $null
  }
}

# ── Download ──────────────────────────────────────────────────────────────────
function Download-Archive {
  param($archiveName, $downloadUrl)

  $tmpDir      = Join-Path $env:TEMP "fastforge-install-$([System.IO.Path]::GetRandomFileName())"
  $archivePath = Join-Path $tmpDir $archiveName

  New-Item -ItemType Directory -Path $tmpDir -Force | Out-Null

  Write-Info "Downloading $archiveName..."
  Write-Info "  from $downloadUrl"

  try {
    $prev = $global:ProgressPreference
    $global:ProgressPreference = 'SilentlyContinue'
    Invoke-WebRequest -Uri $downloadUrl -OutFile $archivePath -UseBasicParsing
    $global:ProgressPreference = $prev
  } catch {
    Remove-Item -Recurse -Force $tmpDir -ErrorAction SilentlyContinue
    Write-Fail "Download failed. Check your network or verify that the release assets exist.`n$_"
  }

  Write-Success "Downloaded $archiveName"
  return @{ TmpDir = $tmpDir; ArchivePath = $archivePath }
}

# ── Install ───────────────────────────────────────────────────────────────────
function Install-Binary {
  param($tmpDir, $archivePath, $version, $target)

  Write-Info "Extracting archive..."
  Expand-Archive -Path $archivePath -DestinationPath $tmpDir -Force

  $binaryPath = Join-Path $tmpDir "$BinaryName-$version-$target\$BinaryName.exe"
  if (-not (Test-Path $binaryPath)) {
    Remove-Item -Recurse -Force $tmpDir -ErrorAction SilentlyContinue
    Write-Fail "Binary not found in archive at expected path: $binaryPath"
  }

  if (-not (Test-Path $InstallDir)) {
    Write-Info "Creating install directory: $InstallDir"
    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
  }

  $dest = Join-Path $InstallDir "$BinaryName.exe"
  Move-Item -Path $binaryPath -Destination $dest -Force
  Remove-Item -Recurse -Force $tmpDir -ErrorAction SilentlyContinue

  Write-Success "Installed $BinaryName to $dest"
}

# ── Update PATH ───────────────────────────────────────────────────────────────
function Update-UserPath {
  $currentPath = [System.Environment]::GetEnvironmentVariable("PATH", "User")
  if ($currentPath -notlike "*$InstallDir*") {
    Write-Info "Adding $InstallDir to your user PATH..."
    $newPath = "$InstallDir;$currentPath"
    [System.Environment]::SetEnvironmentVariable("PATH", $newPath, "User")
    $env:PATH = "$InstallDir;$env:PATH"
    Write-Success "PATH updated. Restart your terminal to apply changes."
  } else {
    Write-Info "$InstallDir is already in your PATH."
  }
}

# ── Verify ────────────────────────────────────────────────────────────────────
function Verify-Install {
  $exePath = Join-Path $InstallDir "$BinaryName.exe"
  if (Test-Path $exePath) {
    try {
      $v = & $exePath --version 2>$null
      Write-Success "Installation verified: $v"
    } catch {
      Write-Success "Binary installed at $exePath"
    }
  } else {
    Write-Warn "Could not verify installation. Binary not found at $exePath"
    return
  }

  $resolvedCmd = Get-Command $BinaryName -ErrorAction SilentlyContinue | Select-Object -First 1
  if (-not $resolvedCmd) {
    Write-Warn "$BinaryName is not on your current PATH. Restart your terminal or add '$InstallDir' to PATH."
    return
  }

  $expectedFull = [System.IO.Path]::GetFullPath($exePath)
  $resolvedSource = if ($resolvedCmd.Source) { $resolvedCmd.Source } else { "$($resolvedCmd.CommandType) '$($resolvedCmd.Name)'" }
  $resolvedFull = try {
    if ($resolvedCmd.Source) { [System.IO.Path]::GetFullPath($resolvedCmd.Source) } else { $null }
  } catch {
    $null
  }

  if (-not $resolvedFull -or -not $resolvedFull.Equals($expectedFull, [System.StringComparison]::OrdinalIgnoreCase)) {
    Write-Warn "Another '$BinaryName' at '$resolvedSource' shadows '$exePath' on your PATH."
    if ($resolvedSource -match 'pub-cache|Pub[\\/]Cache') {
      Write-Warn "This looks like an older Dart installation. Remove it with:"
      Write-Warn "  dart pub global deactivate fastforge"
    } else {
      Write-Warn "Adjust your PATH so '$InstallDir' appears earlier, or remove the shadowing binary (e.g. 'dart pub global deactivate fastforge' if installed via Dart)."
    }
  }
}

# ── Main ──────────────────────────────────────────────────────────────────────
function Main {
  Write-Host ""
  Write-Host "fastforge installer" -ForegroundColor White -BackgroundColor DarkGreen
  Write-Host ""

  $arch    = Get-Arch
  $target  = Get-Target -arch $arch
  $release = Resolve-Release -target $target

  Write-Info "Platform : windows-$arch"
  Write-Info "Target   : $target"
  Write-Info "Version  : $($release.Version)"
  Write-Host ""

  $dl = Download-Archive -archiveName $release.ArchiveName -downloadUrl $release.DownloadUrl
  Install-Binary -tmpDir $dl.TmpDir -archivePath $dl.ArchivePath -version $release.Version -target $target
  Update-UserPath
  Verify-Install

  Write-Host ""
  Write-Host "fastforge $($release.Version) installed successfully!" -ForegroundColor Green
  Write-Host ""
  Write-Host "Run " -NoNewline
  Write-Host "fastforge --help" -ForegroundColor Cyan -NoNewline
  Write-Host " to get started."
  Write-Host ""
}

Main
