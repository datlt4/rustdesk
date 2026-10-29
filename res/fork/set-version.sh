#!/usr/bin/env bash
# Sets the fork version everywhere it is declared; merging the bump into master publishes it.
# Usage: res/fork/set-version.sh 1.5.102 [upstream-version-the-code-is-synced-to]
set -euo pipefail
v=${1:?usage: $0 MAJOR.MINOR.PATCH [UPSTREAM_VERSION]}
up=${2:-}
re='^([0-9]+)\.([0-9]+)\.([0-9]+)$'
[[ $v =~ $re ]] || { echo "not X.Y.Z: $v" >&2; exit 1; }
# Android versionCode must grow with every release.
code=$((BASH_REMATCH[1] * 1000000 + BASH_REMATCH[2] * 1000 + BASH_REMATCH[3]))
cd "$(dirname "$0")/../.."

sed -i "0,/^version = \".*\"/s//version = \"$v\"/" Cargo.toml libs/portable/Cargo.toml
perl -0pi -e "s/(name = \"rustdesk\"\nversion = )\"[^\"]+\"/\${1}\"$v\"/; s/(name = \"rustdesk-portable-packer\"\nversion = )\"[^\"]+\"/\${1}\"$v\"/" Cargo.lock
sed -i "s/^version: .*/version: $v+$code/" flutter/pubspec.yaml
sed -i "s/^  VERSION: \".*\"/  VERSION: \"$v\"/" .github/workflows/flutter-build.yml
sed -i "s/^    version: .*/    version: $v/" appimage/AppImageBuilder-x86_64.yml appimage/AppImageBuilder-aarch64.yml
sed -i "s/^pkgver=.*/pkgver=$v/" res/PKGBUILD
sed -i "s/^Version: .*/Version:    $v/" res/rpm.spec res/rpm-flutter.spec res/rpm-flutter-suse.spec
if [[ -n $up ]]; then
  [[ $up =~ $re ]] || { echo "not X.Y.Z: $up" >&2; exit 1; }
  sed -i "s/^pub const UPSTREAM_VERSION: &str = \".*\";/pub const UPSTREAM_VERSION: \&str = \"$up\";/" src/fork.rs
fi

git --no-pager diff --stat
