# Ubuntu production assets

This directory contains the versioned templates for the recommended production
topology: Cloudflare -> Caddy -> Axum on `127.0.0.1:3100`.

- `canvas-video.service`: hardened systemd service. It contains no credentials.
- `Caddyfile`: TLS and reverse proxy only; access logging and response encoding
  are intentionally absent.
- `production.example.toml`: safe schema example. It cannot start until the
  whitelist placeholder is replaced in `/etc/canvas-video/config.toml`.

Use the scripts in `scripts/` and follow
`docs/deployment-ubuntu-cloudflare.md`. Never put the real production config in
a release directory.
