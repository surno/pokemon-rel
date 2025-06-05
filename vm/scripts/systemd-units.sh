#!/usr/bin/env bash
set -euo pipefail

# --------------------------------------------------------------------------------
# Template service: melon@.service
# --------------------------------------------------------------------------------
cat >/etc/systemd/system/melon@.service <<'EOF'
[Unit]
Description=melonDS instance %i for shiny farm
After=network-online.target

[Service]
Type=simple
User=shinyfarm
# Example ROM/Save path—edit to your own mount or cloud bucket
ExecStart=/usr/local/bin/melonds --bios-path=/opt/melonds/bios --nogui /home/shinyfarm/roms/pokemon.nds
Restart=on-failure
Nice=5

[Install]
WantedBy=multi-user.target
EOF

# --------------------------------------------------------------------------------
# Aggregate target: melon-farm.target
# --------------------------------------------------------------------------------
cat >/etc/systemd/system/melon-farm.target <<'EOF'
[Unit]
Description=Aggregate target to spin up all melonDS shiny-hunting units
Wants=melon@1.service
Wants=melon@2.service
# Add more lines for melon@3.service … melon@n.service

[Install]
WantedBy=multi-user.target
EOF

# Make sure shinyfarm user owns its working dirs
mkdir -p /home/shinyfarm/roms
chown -R shinyfarm:shinyfarm /home/shinyfarm

# Reload systemd daemon and enable the farm target
systemctl daemon-reload
systemctl enable melon-farm.target