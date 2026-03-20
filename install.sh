#!/bin/sh
set -e

REPO="Ashutosh0x/arc-cli"
VERSION=$(curl -s "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name"' | sed -E 's/.*"v([^"]+)".*/\1/')

if [ -z "$VERSION" ]; then
  echo "❌ Failed to fetch latest version. Check https://github.com/${REPO}/releases"
  exit 1
fi

OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

case "$ARCH" in
  x86_64) TARGET="x86_64" ;;
  aarch64|arm64) TARGET="aarch64" ;;
  *) echo "❌ Unsupported architecture: $ARCH"; exit 1 ;;
esac

case "$OS" in
  linux) TRIPLE="${TARGET}-unknown-linux-musl" ;;
  darwin) TRIPLE="${TARGET}-apple-darwin" ;;
  *) echo "❌ Unsupported OS: $OS. Use install.ps1 for Windows."; exit 1 ;;
esac

URL="https://github.com/${REPO}/releases/download/v${VERSION}/arc-v${VERSION}-${TRIPLE}.tar.gz"
INSTALL_DIR="${ARC_INSTALL_DIR:-/usr/local/bin}"

echo "⚡ Installing ARC CLI v${VERSION} for ${TRIPLE}..."
echo "   Downloading from: ${URL}"

if [ -w "$INSTALL_DIR" ]; then
  curl -fsSL "$URL" | tar xz -C "$INSTALL_DIR" arc
else
  echo "   (requires sudo for $INSTALL_DIR)"
  curl -fsSL "$URL" | sudo tar xz -C "$INSTALL_DIR" arc
fi

echo "✅ arc v${VERSION} installed to ${INSTALL_DIR}/arc"
echo ""
echo "   Run 'arc chat' to start an autonomous session."
