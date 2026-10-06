#!/bin/sh
# install.sh — deploy EasyTier to /etc/rc.d on OpenBSD (run as root)
set -e

SRC_DIR=$(cd "$(dirname "$0")" && pwd)
DEST_BASE=/etc/easytier

# binaries
install -m 755 "${SRC_DIR}/../target/release/easytier-core" /usr/local/bin/easytier-core
install -m 755 "${SRC_DIR}/../target/release/easytier-cli"  /usr/local/bin/easytier-cli
install -m 755 "${SRC_DIR}/../target/release/easytier-web"  /usr/local/bin/easytier-web

# rc.d
install -m 555 "${SRC_DIR}/rc.d.easytier"    /etc/rc.d/easytier
install -m 555 "${SRC_DIR}/rc.d.easytierweb" /etc/rc.d/easytierweb

# config & dirs
mkdir -p "${DEST_BASE}" /var/db/easytier /var/log/easytier
install -m 644 "${SRC_DIR}/config.toml" "${DEST_BASE}/config.toml"

rcctl enable easytier easytierweb
rcctl start  easytier easytierweb
echo "Done. Web UI: http://$(hostname):11211/"
