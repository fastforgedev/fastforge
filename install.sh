#!/bin/sh
set -e

REPO="fastforgedev/fastforge"
BINARY_NAME="fastforge"
INSTALL_DIR="${FASTFORGE_INSTALL_DIR:-/usr/local/bin}"

# ── Terminal colors ────────────────────────────────────────────────────────────
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
BOLD='\033[1m'
RESET='\033[0m'

info()    { printf "${CYAN}info${RESET}  %s\n" "$1"; }
success() { printf "${GREEN}✓${RESET}     %s\n" "$1"; }
warn()    { printf "${YELLOW}warn${RESET}  %s\n" "$1"; }
error()   { printf "${RED}error${RESET} %s\n" "$1" >&2; exit 1; }

# ── Detect OS / arch ──────────────────────────────────────────────────────────
detect_platform() {
  OS="$(uname -s)"
  ARCH="$(uname -m)"

  case "$OS" in
    Linux)  OS_NAME="linux" ;;
    Darwin) OS_NAME="darwin" ;;
    *)      error "Unsupported operating system: $OS" ;;
  esac

  case "$ARCH" in
    x86_64 | amd64)  ARCH_NAME="x86_64" ;;
    aarch64 | arm64) ARCH_NAME="aarch64" ;;
    *) error "Unsupported architecture: $ARCH" ;;
  esac

  PLATFORM="${OS_NAME}-${ARCH_NAME}"
}

# ── Map platform to Rust target triple ────────────────────────────────────────
resolve_target() {
  case "$PLATFORM" in
    darwin-aarch64) TARGET="aarch64-apple-darwin" ;;
    darwin-x86_64)  TARGET="x86_64-apple-darwin" ;;
    linux-aarch64)  TARGET="aarch64-unknown-linux-gnu" ;;
    linux-x86_64)   TARGET="x86_64-unknown-linux-gnu" ;;
    *) error "No prebuilt binary available for platform: $PLATFORM" ;;
  esac
}

# ── Fetch release JSON from GitHub API ────────────────────────────────────────
fetch_releases() {
  API_URL="https://api.github.com/repos/${REPO}/releases?per_page=100"

  if [ -n "$GITHUB_TOKEN" ]; then
    if command -v curl >/dev/null 2>&1; then
      RELEASE_JSON="$(curl -fsSL -H "Authorization: Bearer ${GITHUB_TOKEN}" "$API_URL")"
    elif command -v wget >/dev/null 2>&1; then
      RELEASE_JSON="$(wget -qO- --header="Authorization: Bearer ${GITHUB_TOKEN}" "$API_URL")"
    else
      error "Neither curl nor wget is available. Please install one and retry."
    fi
  else
    if command -v curl >/dev/null 2>&1; then
      RELEASE_JSON="$(curl -fsSL "$API_URL")"
    elif command -v wget >/dev/null 2>&1; then
      RELEASE_JSON="$(wget -qO- "$API_URL")"
    else
      error "Neither curl nor wget is available. Please install one and retry."
    fi
  fi

  if [ -z "$RELEASE_JSON" ]; then
    error "Failed to fetch release information from GitHub API."
  fi
}

# ── Resolve version + download URL ────────────────────────────────────────────
resolve_release() {
  if [ -n "$FASTFORGE_VERSION" ]; then
    VERSION="$FASTFORGE_VERSION"
    info "Using specified version: $VERSION"

    # When version is manually specified, fall back to constructing the URL
    ARCHIVE_NAME="${BINARY_NAME}-${VERSION}-${TARGET}.tar.gz"
    DOWNLOAD_URL="https://github.com/${REPO}/releases/download/v${VERSION}/${ARCHIVE_NAME}"
    return
  fi

  info "Fetching latest release version..."
  fetch_releases

  # Pick the newest published (non-draft, non-prerelease) release that ships
  # a binary for this target (releases are listed newest first; releases from
  # the Dart era carry no binaries).
  MATCHED_RELEASE="$(printf '%s\n' "$RELEASE_JSON" \
    | awk -v prefix="${BINARY_NAME}-" -v suffix="-${TARGET}.tar.gz" '
        function reset_release() {
          is_draft = 0; is_pre = 0; in_assets = 0
          found_name = ""; found_dl = ""; found_api = ""
        }
        function reset_asset() {
          a_name = ""; a_dl = ""; a_api = ""
        }
        function matches_archive(name,    plen, slen, nlen) {
          plen = length(prefix); slen = length(suffix); nlen = length(name)
          return (nlen > plen + slen &&
                  substr(name, 1, plen) == prefix &&
                  substr(name, nlen - slen + 1, slen) == suffix)
        }
        {
          line = $0
          n = length(line)
          i = 1
          while (i <= n) {
            c = substr(line, i, 1)
            if (c == " " || c == "\t" || c == "\r" || c == "\n") {
              i++
              continue
            }
            if (c == "{") {
              sp++; container[sp] = "{"
              depth++
              after_colon[depth] = 0
              obj_key[depth] = ""
              if (depth == 1) reset_release()
              else if (in_assets && depth == 2) reset_asset()
              i++
            } else if (c == "}") {
              if (in_assets && depth == 2 && found_name == "" && matches_archive(a_name)) {
                found_name = a_name
                found_dl = a_dl
                found_api = a_api
              } else if (depth == 1 && !is_draft && !is_pre && found_name != "") {
                print found_name "|" found_dl "|" found_api
                exit 0
              }
              depth--
              sp--
              i++
            } else if (c == "[") {
              sp++; container[sp] = "["
              arr_depth++
              if (depth == 1 && after_colon[1] && obj_key[1] == "assets") {
                in_assets = 1
                assets_arr_depth = arr_depth
              }
              i++
            } else if (c == "]") {
              if (in_assets && arr_depth == assets_arr_depth) {
                in_assets = 0
              }
              arr_depth--
              sp--
              i++
            } else if (c == ":") {
              if (container[sp] == "{") after_colon[depth] = 1
              i++
            } else if (c == ",") {
              if (container[sp] == "{") {
                after_colon[depth] = 0
                obj_key[depth] = ""
              }
              i++
            } else if (c == "\"") {
              capture = (container[sp] == "{" &&
                         ((depth == 1 && !after_colon[1]) ||
                          (in_assets && depth == 2 &&
                           (!after_colon[2] || obj_key[2] == "name" ||
                            obj_key[2] == "browser_download_url" || obj_key[2] == "url"))))
              str = ""
              i++
              while (i <= n) {
                ch = substr(line, i, 1)
                if (ch == "\\") {
                  i++
                  if (capture && i <= n) str = str substr(line, i, 1)
                } else if (ch == "\"") {
                  break
                } else if (capture) {
                  str = str ch
                }
                i++
              }
              i++
              if (container[sp] == "{") {
                if (!after_colon[depth]) {
                  obj_key[depth] = str
                } else if (in_assets && depth == 2) {
                  if (obj_key[2] == "name") a_name = str
                  else if (obj_key[2] == "browser_download_url") a_dl = str
                  else if (obj_key[2] == "url") a_api = str
                }
              }
            } else {
              tok = ""
              while (i <= n) {
                ch = substr(line, i, 1)
                if (ch == " " || ch == "\t" || ch == "\r" || ch == "\n" ||
                    ch == "," || ch == "}" || ch == "]" || ch == ":") {
                  break
                }
                tok = tok ch
                i++
              }
              if (container[sp] == "{" && after_colon[depth] && depth == 1) {
                if (obj_key[1] == "draft") is_draft = (tok == "true")
                else if (obj_key[1] == "prerelease") is_pre = (tok == "true")
              }
            }
          }
        }
      ')"

  ARCHIVE_NAME=""
  DOWNLOAD_URL=""
  ASSET_API_URL=""
  if [ -n "$MATCHED_RELEASE" ]; then
    IFS='|' read -r ARCHIVE_NAME DOWNLOAD_URL ASSET_API_URL <<EOF
$MATCHED_RELEASE
EOF
  fi

  if [ -z "$ARCHIVE_NAME" ]; then
    error "No published release provides a prebuilt binary for ${TARGET}. Set FASTFORGE_VERSION to specify one manually."
  fi

  VERSION="${ARCHIVE_NAME#"${BINARY_NAME}-"}"
  VERSION="${VERSION%"-${TARGET}.tar.gz"}"

  info "Latest version: $VERSION"

  if [ -z "$DOWNLOAD_URL" ] && [ -z "$ASSET_API_URL" ]; then
    error "No download asset found for target ${TARGET} in the latest release. The release may not have finished uploading assets yet."
  fi
}

# ── Download ──────────────────────────────────────────────────────────────────
download() {
  TMP_DIR="$(mktemp -d)"
  ARCHIVE_PATH="${TMP_DIR}/${ARCHIVE_NAME}"

  # Use API asset URL with token for draft releases, otherwise use browser_download_url
  if [ -n "$GITHUB_TOKEN" ] && [ -n "$ASSET_API_URL" ]; then
    info "Downloading ${ARCHIVE_NAME} (via API)..."
    info "  from ${ASSET_API_URL}"
    if command -v curl >/dev/null 2>&1; then
      curl -fsSL --progress-bar \
        -H "Authorization: Bearer ${GITHUB_TOKEN}" \
        -H "Accept: application/octet-stream" \
        "$ASSET_API_URL" -o "$ARCHIVE_PATH" || \
        error "Download failed. Check your network or verify that version v${VERSION} exists."
    elif command -v wget >/dev/null 2>&1; then
      wget -q --show-progress \
        --header="Authorization: Bearer ${GITHUB_TOKEN}" \
        --header="Accept: application/octet-stream" \
        "$ASSET_API_URL" -O "$ARCHIVE_PATH" 2>&1 || \
        error "Download failed. Check your network or verify that version v${VERSION} exists."
    fi
  else
    info "Downloading ${ARCHIVE_NAME}..."
    info "  from ${DOWNLOAD_URL}"
    if command -v curl >/dev/null 2>&1; then
      curl -fsSL --progress-bar "$DOWNLOAD_URL" -o "$ARCHIVE_PATH" || \
        error "Download failed. Check your network or verify that version v${VERSION} exists."
    elif command -v wget >/dev/null 2>&1; then
      wget -q --show-progress "$DOWNLOAD_URL" -O "$ARCHIVE_PATH" 2>&1 || \
        error "Download failed. Check your network or verify that version v${VERSION} exists."
    fi
  fi

  success "Downloaded ${ARCHIVE_NAME}"
}

# ── Install ───────────────────────────────────────────────────────────────────
install_binary() {
  info "Extracting archive..."
  tar -xzf "$ARCHIVE_PATH" -C "$TMP_DIR"

  BINARY_PATH="${TMP_DIR}/${BINARY_NAME}-${VERSION}-${TARGET}/${BINARY_NAME}"

  if [ ! -f "$BINARY_PATH" ]; then
    error "Binary not found in archive at expected path: ${BINARY_PATH}"
  fi

  chmod +x "$BINARY_PATH"

  if [ ! -d "$INSTALL_DIR" ]; then
    info "Creating install directory: ${INSTALL_DIR}"
    mkdir -p "$INSTALL_DIR" 2>/dev/null || sudo mkdir -p "$INSTALL_DIR"
  fi

  if [ -w "$INSTALL_DIR" ]; then
    mv "$BINARY_PATH" "${INSTALL_DIR}/${BINARY_NAME}"
  else
    info "Requesting elevated permissions to install into ${INSTALL_DIR}..."
    sudo mv "$BINARY_PATH" "${INSTALL_DIR}/${BINARY_NAME}"
  fi

  rm -rf "$TMP_DIR"

  success "Installed ${BINARY_NAME} to ${INSTALL_DIR}/${BINARY_NAME}"
}

# ── Verify ────────────────────────────────────────────────────────────────────
verify() {
  EXPECTED_PATH="${INSTALL_DIR}/${BINARY_NAME}"
  INSTALLED_VERSION="$("${EXPECTED_PATH}" --version 2>/dev/null || true)"
  RESOLVED_PATH="$(command -v "$BINARY_NAME" 2>/dev/null || true)"

  if [ -z "$RESOLVED_PATH" ]; then
    if [ -n "$INSTALLED_VERSION" ]; then
      success "Binary verified at ${EXPECTED_PATH}: ${INSTALLED_VERSION}"
    fi
    warn "${BINARY_NAME} is not on your PATH."
    warn "Add the following line to your shell profile (~/.bashrc, ~/.zshrc, etc.):"
    warn "  export PATH=\"${INSTALL_DIR}:\$PATH\""
    return
  fi

  EXPECTED_DIR="$(cd "$(dirname "$EXPECTED_PATH")" 2>/dev/null && pwd -P || dirname "$EXPECTED_PATH")"
  RESOLVED_DIR="$(cd "$(dirname "$RESOLVED_PATH")" 2>/dev/null && pwd -P || dirname "$RESOLVED_PATH")"
  EXPECTED_CANON="${EXPECTED_DIR}/$(basename "$EXPECTED_PATH")"
  RESOLVED_CANON="${RESOLVED_DIR}/$(basename "$RESOLVED_PATH")"

  if [ "$RESOLVED_CANON" != "$EXPECTED_CANON" ]; then
    if [ -n "$INSTALLED_VERSION" ]; then
      success "Binary verified at ${EXPECTED_PATH}: ${INSTALLED_VERSION}"
    fi
    warn "Another '${BINARY_NAME}' at ${RESOLVED_PATH} shadows ${EXPECTED_PATH} on your PATH."
    case "$RESOLVED_PATH" in
      *.pub-cache*)
        warn "This looks like an older Dart installation. Remove it with:"
        warn "  dart pub global deactivate fastforge"
        ;;
      *)
        warn "Adjust your PATH so ${INSTALL_DIR} appears earlier, or remove the shadowing binary (e.g. 'dart pub global deactivate fastforge' if installed via Dart)."
        ;;
    esac
  else
    success "Installation verified: ${INSTALLED_VERSION}"
  fi
}

# ── Main ──────────────────────────────────────────────────────────────────────
main() {
  printf "\n${BOLD}fastforge installer${RESET}\n\n"

  detect_platform
  resolve_target
  resolve_release
  download
  install_binary
  verify

  printf "\n${BOLD}${GREEN}fastforge ${VERSION} installed successfully!${RESET}\n\n"
  printf "Run ${CYAN}fastforge --help${RESET} to get started.\n\n"
}

main
