#!/bin/sh
# Installs or updates Portkeeper on macOS or Linux, without the Gatekeeper dialog.
#   curl -fsSL https://portkeeper.pages.dev/install.sh | sh
# Downloads the latest release from GitHub over HTTPS and, when minisign is installed,
# checks its signature against Portkeeper's release key (the same key the in-app
# updater trusts). Files fetched by curl carry no quarantine flag, so macOS does not
# show the "could not verify" dialog. Later updates come from inside the app.
# Source: https://github.com/MarcAparisiTFG/portkeeper
# Windows: irm https://portkeeper.pages.dev/install.ps1 | iex
#
# Options (environment variables):
#   PORTKEEPER_REQUIRE_SIG=1   fail instead of continuing when minisign is missing
#   PORTKEEPER_APP_DIR=~/Applications   macOS: install somewhere other than /Applications
#   PORTKEEPER_NO_LAUNCH=1     do not open the app afterwards
set -eu

REPO="https://github.com/MarcAparisiTFG/portkeeper"
# Portkeeper's release public key (minisign). Also in src-tauri/tauri.conf.json, base64-wrapped.
PUBKEY="RWR66DOwDY9T5TW18Ponp86IqRfzFjGfws9eQtESollZR4XU58rTaEpq"

fetch() { # url, output
  curl --proto '=https' --tlsv1.2 -fsSL "$1" -o "$2"
}

# Check a release asset against its published .sig (a base64-wrapped minisign signature).
verify() { # file, sig
  if ! command -v minisign >/dev/null 2>&1; then
    if [ "${PORTKEEPER_REQUIRE_SIG:-}" ]; then
      echo "minisign is not installed, so the signature cannot be checked (brew install minisign)."; exit 1
    fi
    echo "Note: install minisign (brew install minisign) and the next run will check the download's signature."
    return 0
  fi
  base64 --decode < "$2" > "$2.minisig" || { echo "The signature file is not valid."; exit 1; }
  if ! minisign -Vq -m "$1" -x "$2.minisig" -P "$PUBKEY"; then
    echo "SIGNATURE CHECK FAILED: the download was not signed with Portkeeper's release key. Not installing."
    exit 1
  fi
  echo "Signature OK."
}

main() {
  TMP="$(mktemp -d)"
  trap 'rm -rf "$TMP"' EXIT

  # Resolve the latest version from the "latest" redirect: the signed assets carry it in
  # their names (tauri-action 1.0 also versions the macOS .app.tar.gz).
  TAG="$(curl --proto '=https' --tlsv1.2 -fsSIL -o /dev/null -w '%{url_effective}' "$REPO/releases/latest")"
  TAG="${TAG##*/}"; VER="${TAG#v}"
  [ "$VER" != "$TAG" ] && [ -n "$VER" ] || { echo "Could not find the latest release."; exit 1; }

  case "$(uname -s)" in
    Darwin)
      case "$(uname -m)" in
        arm64) ARCH=aarch64 ;;
        x86_64) ARCH=x64 ;;
        *) echo "Unsupported CPU: $(uname -m)"; exit 1 ;;
      esac
      APP_DIR="${PORTKEEPER_APP_DIR:-/Applications}"
      APP="$APP_DIR/Portkeeper.app"
      echo "Downloading Portkeeper $VER for macOS ($ARCH)…"
      # Releases built with tauri-action 1.0 name the archive Portkeeper_<ver>_<arch>.app.tar.gz;
      # older ones Portkeeper_<arch>.app.tar.gz. Try the new name, then the old one.
      for f in app.tar.gz app.tar.gz.sig; do
        fetch "$REPO/releases/download/$TAG/Portkeeper_${VER}_${ARCH}.$f" "$TMP/$f" 2>/dev/null \
          || fetch "$REPO/releases/download/$TAG/Portkeeper_${ARCH}.$f" "$TMP/$f"
      done
      verify "$TMP/app.tar.gz" "$TMP/app.tar.gz.sig"
      tar -xzf "$TMP/app.tar.gz" -C "$TMP"
      [ -d "$TMP/Portkeeper.app" ] || { echo "Unexpected archive layout"; exit 1; }
      # Stage next to the final location first, so the old app is only removed once the
      # new one is fully in place; the final step is a rename on the same volume.
      mkdir -p "$APP_DIR"
      rm -rf "$APP.new"
      mv "$TMP/Portkeeper.app" "$APP.new"
      if pgrep -fq "$APP/Contents/MacOS/portkeeper"; then
        echo "Closing the running Portkeeper…"
        pkill -f "$APP/Contents/MacOS/portkeeper" || true
        sleep 1
      fi
      rm -rf "$APP"
      mv "$APP.new" "$APP"
      xattr -cr "$APP" 2>/dev/null || true
      VERSION="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' "$APP/Contents/Info.plist" 2>/dev/null || echo '?')"
      if [ "${PORTKEEPER_NO_LAUNCH:-}" ]; then
        echo "Portkeeper $VERSION is installed at $APP."
      else
        open "$APP"; echo "Portkeeper $VERSION is installed and running in your menu bar."
      fi
      ;;
    Linux)
      # Tauri names the AppImage after the Debian architecture: amd64, aarch64.
      case "$(uname -m)" in
        x86_64 | amd64) ARCH=x86_64; ASSET=amd64 ;;
        aarch64 | arm64) ARCH=aarch64; ASSET=aarch64 ;;
        *) echo "Only x86_64 and aarch64 Linux builds exist for now: $REPO/releases"; exit 1 ;;
      esac
      BIN="$HOME/.local/bin"; APPS="$HOME/.local/share/applications"
      mkdir -p "$BIN" "$APPS"
      echo "Downloading Portkeeper $VER for Linux ($ARCH)…"
      fetch "$REPO/releases/download/$TAG/Portkeeper_${VER}_${ASSET}.AppImage" "$TMP/portkeeper" \
        || { echo "Portkeeper $VER has no Linux $ARCH build: $REPO/releases"; exit 1; }
      fetch "$REPO/releases/download/$TAG/Portkeeper_${VER}_${ASSET}.AppImage.sig" "$TMP/portkeeper.sig"
      verify "$TMP/portkeeper" "$TMP/portkeeper.sig"
      chmod +x "$TMP/portkeeper"
      pkill -f "$BIN/portkeeper" 2>/dev/null || true
      mv "$TMP/portkeeper" "$BIN/portkeeper"
      cat > "$APPS/portkeeper.desktop" <<DESKTOP
[Desktop Entry]
Type=Application
Name=Portkeeper
Comment=See the ports you have open and who started them
Exec="$BIN/portkeeper"
Terminal=false
Categories=Development;Utility;
DESKTOP
      if [ "${PORTKEEPER_NO_LAUNCH:-}" ]; then
        echo "Portkeeper $VER is installed at $BIN/portkeeper."
      else
        (nohup "$BIN/portkeeper" >/dev/null 2>&1 &); echo "Portkeeper $VER is installed at $BIN/portkeeper and running in your tray."
      fi
      case ":$PATH:" in *":$BIN:"*) ;; *) echo "Tip: add $BIN to your PATH to run it as 'portkeeper'." ;; esac
      ;;
    *)
      echo "This installer is for macOS and Linux. Windows: irm https://portkeeper.pages.dev/install.ps1 | iex"; exit 1 ;;
  esac
  echo "Updates arrive inside the app. Settings, locks and the Closed list are kept."
}

# Everything above only defines functions; nothing runs until this line, so a download
# that is cut short cannot execute a half-received script.
main "$@"
