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
variable "cpus" {
  type    = number
  default = 8
}

variable "ram" {
  type    = number
  default = 8192  # MiB
}

variable "disk_size" {
  type    = string
  default = "20G"
}

# ✅  SHA-256 for debian-12.11.0-amd64-netinst.iso  (May 2025 mirrors)
#    30ca12a15cae6a1033e03ad59eb7f66a6d5a258dcf27acd115c2bd42d22640e8
variable "iso_checksum"  {
  type    = string
  default = "sha256:30ca12a15cae6a1033e03ad59eb7f66a6d5a258dcf27acd115c2bd42d22640e8"
}

source "qemu" "debian12" {
  accelerator   = "tcg"                     # keep x86 for libTAS compatibility
  qemu_binary   = "qemu-system-x86_64"

  iso_url       = "https://cdimage.debian.org/debian-cd/current/amd64/iso-cd/debian-12.11.0-amd64-netinst.iso"
  iso_checksum  = var.iso_checksum

  vm_name       = "shinyfarm-base-tmp"
  output_directory = "output/qcow"
  disk_size     = var.disk_size
  format        = "qcow2"

  headless      = true
  shutdown_timeout = "10m"
  shutdown_command = "sudo shutdown -P now"

  http_directory = "http"           # contains preseed.cfg
  ssh_username   = "shinyfarm"
  ssh_password   = "SuperSecret!"
  ssh_timeout    = "1h"

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
    "locale=en_US keyboard-configuration/xkb-keymap=us ",
    "hostname=shinyfarm ",
    "fb=false debconf/frontend=noninteractive ",
    "<enter>"
  ]
}

build {
  sources = ["source.qemu.debian12"]

  # Minimal provisioning (just confirm sudo works)
  provisioner "shell" {
    inline = ["echo 'golden image ready'"]
    execute_command = "sudo -E sh -c '{{ .Vars }} {{ .Path }}'"
  }

post-processor "artifice" {
  files               = ["**/*.qcow2", "**/*.qcow", "**/shinyfarm-base-tmp"]   # recurse just in case
  keep_input_artifact = true
}

post-processor "shell-local" {
  inline = [
    "set -euxo pipefail",
    "echo 'Current dir:' $(pwd)",
    "ls -alR",                           # full recursive listing
    # search recursively inside *this* artifact dir
    "qcow=$(find . -type f \\( -name '*.qcow2' -o -name '*.qcow' -o -name 'shinyfarm-base-tmp' \\) | head -n1)",
    # last-chance fall-back to original output dir
    "if [ -z \"$qcow\" ]; then qcow=$(find ../output/qcow -type f \\( -name '*.qcow2' -o -name '*.qcow' -o -name 'shinyfarm-base-tmp' \\) | head -n1 || true); fi",
    "if [ -z \"$qcow\" ]; then echo 'No qcow image found anywhere'; exit 1; fi",
    "echo \"Copying $qcow -> ../shinyfarm-base.qcow2\"",
    "cp \"$qcow\" ../shinyfarm-base.qcow2"
  ]
}
}