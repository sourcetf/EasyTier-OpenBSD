# EasyTier on OpenBSD — Deployment Guide

## Runtime Dependencies
pkg_add -I rust git protobuf llvm

## Build
    git clone https://github.com/sourcetf/EasyTier-OpenBSD.git
    cd EasyTier-OpenBSD
    (cd easytier-web/frontend && pnpm install --prefer-offline && pnpm build)
    cargo build --release --ignore-rust-version \
      -p easytier -p easytier-web --bins --features easytier-web/embed
    strip target/release/easytier-{core,cli,web}
    install -m 755 target/release/easytier-{core,cli,web} /usr/local/bin/

## Deploy (as root)
    install -m 555 deploy/openbsd/rc.d.easytier     /etc/rc.d/easytier
    install -m 555 deploy/openbsd/rc.d.easytierweb  /etc/rc.d/easytierweb
    install -m 644 deploy/openbsd/config.toml       /etc/easytier/config.toml
    mkdir -p /etc/easytier /var/db/easytier /var/log/easytier
    rcctl enable easytier easytierweb
    rcctl start  easytier easytierweb

## Binaries & Ports
    /usr/local/bin/easytier-{core,cli,web}
    Web UI:       http://<host>:11211/   (register admin in the UI)
    RPC portal:   easytier-cli --rpc-portal 127.0.0.1:15888 peer list
    Listeners:    11010-11013  TCP/UDP/wg/quic/ws/wss/faketcp
                 15888         easytier-core RPC
                 11211         easytier-web
                 22020         config-server (--config-server mode)

## Logs
    tail -f /var/log/daemon              # easytier-core -> syslog
    ls -la /var/log/easytier/           # easytier-web file log

## Join a Different Mesh
Edit [network_identity] in /etc/easytier/config.toml (network_name / network_secret),
add peers under [flags] (e.g. peers = ["tcp://<remote-ip>:11010"]) and:
    rcctl restart easytier
