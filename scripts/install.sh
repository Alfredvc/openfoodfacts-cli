#!/usr/bin/env bash
set -euo pipefail

REPO="alfredvc/openfoodfacts-cli"
INSTALL_DIR="${HOME}/.local/bin"
BINARY="openfoodfacts"

# Detect OS and arch
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

case "$OS" in
  linux)  OS_NAME="linux" ;;
  darwin) OS_NAME="macos" ;;
  *) echo "Unsupported OS: $OS" >&2; exit 1 ;;
esac

case "$ARCH" in
  x86_64)  ARCH_NAME="x86_64" ;;
  aarch64|arm64) ARCH_NAME="aarch64" ;;
  *) echo "Unsupported architecture: $ARCH" >&2; exit 1 ;;
esac

ARTIFACT="${BINARY}-${OS_NAME}-${ARCH_NAME}"
LATEST=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name"' | sed 's/.*"tag_name": "\(.*\)".*/\1/')
URL="https://github.com/${REPO}/releases/download/${LATEST}/${ARTIFACT}.tar.gz"

echo "Installing ${BINARY} ${LATEST} for ${OS_NAME}/${ARCH_NAME}..."
mkdir -p "$INSTALL_DIR"
if ! curl -fsSL "$URL" | tar -xz -C "$INSTALL_DIR" "$BINARY"; then
  echo "Error: Failed to download or extract ${ARTIFACT}.tar.gz" >&2
  echo "Check that a release exists for ${OS_NAME}/${ARCH_NAME}" >&2
  exit 1
fi
chmod +x "${INSTALL_DIR}/${BINARY}"
echo "Installed to ${INSTALL_DIR}/${BINARY}"

if [[ ":$PATH:" != *":${INSTALL_DIR}:"* ]]; then
  echo "Note: Add ${INSTALL_DIR} to your PATH"
fi
