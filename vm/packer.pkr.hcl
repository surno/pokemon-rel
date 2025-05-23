packer {
  required_plugins {
    qemu = {
      version = ">= 1.1.0"
      source  = "github.com/hashicorp/qemu"
    }
  }
}

###############################################################################
# ─── VARIABLES ──────────────────────────────────────────────────────────────
###############################################################################
variable "vm_count" {
  type    = number
  default = 1
  # fleet size
}

variable "cpus" {
  type    = number
  default = 8
  # per‑VM vCPUs
}

variable "ram" {
  type    = number
  default = 8192
  # MiB per VM
}

variable "disk_size" {
  type    = string
  default = "20G"
}

variable "host_ssh_baseport" {
  type    = number
  default = 2222
}

# SHA-256 for debian-12.11.0-amd64-netinst.iso  (mirrors May 2025)
#   30ca12a15cae6a1033e03ad59eb7f66a6d5a258dcf27acd115c2bd42d22640e8
variable "iso_checksum" {
  type    = string
  default = "sha256:30ca12a15cae6a1033e03ad59eb7f66a6d5a258dcf27acd115c2bd42d22640e8"
}

source "qemu" "debian12" {
  accelerator     = "tcg"
  qemu_binary     = "qemu-system-x86_64"
  iso_url         = "https://cdimage.debian.org/debian-cd/current/amd64/iso-cd/debian-12.11.0-amd64-netinst.iso"
  iso_checksum     = var.iso_checksum

  vm_name         = "shinyfarm"
  output_directory= "output/qcow"
  disk_size       = var.disk_size
  format          = "qcow2"

  headless        = true
  shutdown_timeout= "10m"
  shutdown_command = "sudo shutdown -P now"

  http_directory  = "http"
  ssh_username    = "shinyfarm"
  ssh_password    = "SuperSecret!"
  ssh_timeout     = "1h"

  qemuargs = [
    ["-smp", "${var.cpus}"],
    ["-m",   "${var.ram}"],
    ["-net", "nic,model=virtio"],
    ["-net", "user"]
  ]

  boot_command = [
    "<wait5><esc><wait>",
    "install netcfg/choose_interface=auto ",
    "auto=true priority=critical ",
    "preseed/url=http://{{ .HTTPIP }}:{{ .HTTPPort }}/preseed.cfg ",
    "locale=en_US ",
    "keyboard-configuration/xkb-keymap=us ",
    "hostname=shinyfarm ",
    "fb=false debconf/frontend=noninteractive ",
    "<enter>"
  ]
}

build {
  sources = ["source.qemu.debian12"]

  provisioner "shell" {
    scripts = [
      "scripts/build-deps.sh",
      "scripts/build-melonds.sh",
      "scripts/build-libtas.sh",
      "scripts/systemd-units.sh"
    ]
    # run every script via sudo so apt can touch system paths
    execute_command = "sudo -E sh -c '{{ .Vars }} {{ .Path }}'"  
  }

  # ── cloud-init seed ISO bundled into artifact ────────────────────────────
  post-processor "shell-local" {
    inline = [
      "echo 'instance-id: shinyfarm' > meta-data",
      "cloud-localds --network-config=network-config.yml user-seed.iso user-data.yaml meta-data"
    ]
  }
}