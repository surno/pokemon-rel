#!/usr/bin/env bash
set -euo pipefail

cat >/etc/systemd/system/melon@\x2eservice <<'EOF'
[Unit]
Description=melonDS instance %i for shiny farm
After=network-online.target

[Service]
Type=simple
User=packer
# Example ROM/Save path—edit to your own mount or cloud bucket
ExecStart=/usr/local/bin/melonds --bios-path=/opt/melonds/bios --nogui /home/packer/roms/pokemon.nds
Restart=on-failure
Nice=5

[Install]
WantedBy=multi-user.target
EOF

# A target that can pull as many parallel melonDS services as you like
cat >/etc/systemd/system/melon-farm.target <<'EOF'
[Unit]
Description=Aggregate target to spin up all melonDS shiny-hunting units
Wants=melon@1.service
Wants=melon@2.service
# Add more lines for melon@3.service … melon@n.service
EOF

# Make sure packer user owns its working dirs
mkdir -p /home/packer/roms
chown -R packer:packer /home/packer

systemctl daemon-reload
systemctl enable melon-farm.target