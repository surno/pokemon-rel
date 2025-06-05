###############################################################################
# shinyfarm-app.pkr.hcl  –  workload/overlay layer
# Requires: shinyfarm-base.qcow2 (built once by the base template)
###############################################################################

packer {
  required_plugins {
    qemu = {
      version = ">= 1.1.0"
      source  = "github.com/hashicorp/qemu"
    }
  }
}

variable "cpus" {
  type    = number
  default = 8
}

variable "ram" {
  type    = number
  default = 8192
}

variable "base_image" {
  type    = string
  default = "../shinyfarm-base.qcow2"   # path to the golden base image
}

source "qemu" "overlay" {

  qemu_binary  = "qemu-system-x86_64"
  iso_url        = var.base_image            # treat qcow2 as bootable image
  iso_checksum   = "none"
  disk_image       = true                 # iso_url is a disk image, not optical
  use_backing_file = true              # create new overlay referencing base
  headless     = true

  ssh_username = "shinyfarm"
  ssh_password = "SuperSecret!"
  ssh_timeout  = "1h"

  qemuargs = [
    ["-smp", "${var.cpus}"],
    ["-m",   "${var.ram}"],
    ["-net", "nic,model=virtio"],
    ["-net", "user"]
  ]
}

build {
  sources = ["source.qemu.overlay"]

  provisioner "shell" {
    scripts = [
      "scripts/build-deps.sh",
      "scripts/build-melonds.sh",
      "scripts/build-libtas.sh",
      "scripts/systemd-units.sh"
    ]
    execute_command = "sudo -E sh -c '{{ .Vars }} {{ .Path }}'"
  }

  post-processor "shell-local" {
    inline = [
      "echo 'instance-id: shinyfarm' > meta-data"
    ]
  }
}