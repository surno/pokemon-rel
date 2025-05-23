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
variable "vm_count"          { type = number default = 1 }   # fleet size
variable "cpus"              { type = number default = 2  }  # per-VM vCPUs
variable "ram"               { type = number default = 4096 }# MiB per VM
variable "disk_size"         { type = string  default = "20G" }
variable "host_ssh_baseport" { type = number default = 2222 }

# SHA-256 for debian-12.5.0-amd64-netinst.iso  (mirrors Feb 2025)
#   1eef148d89ef4edefbc968453c12035fa7911f3e3f2eb0ec5fc1f9c0d43ea63d
variable "iso_checksum" {
  type    = string
  default = "sha256:1eef148d89ef4edefbc968453c12035fa7911f3e3f2eb0ec5fc1f9c0d43ea63d"
}

source "qemu" "debian12" {
  accelerator     = "hvf"                    
  iso_url         = "https://cdimage.debian.org/debian-cd/current/amd64/iso-cd/debian-12.5.0-amd64-netinst.iso"
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
  ssh_password    = "shinyfarm"
  ssh_timeout     = "20m"

  # dynamic host-side SSH port → 2222, 2223, … one per VM build
  qemuargs = [
    ["-smp", "${var.cpus}"],
    ["-m",  "${var.ram}"],
    ["-net", "nic,model=virtio"],
    ["-net", "user,hostfwd=tcp::${var.host_ssh_baseport + build.Index}-:22"],
    ["-drive", "file=user-seed.iso,format=raw,if=virtio"]
  ]

  boot_command = [
    "<enter><wait>",
    "auto url=http://{{ .HTTPIP }}:{{ .HTTPPort }}/preseed.cfg ",
    "debian-installer=en_US ",
    "hostname=shinyfarm ",
    "fb=false debconf/frontend=noninteractive<enter>"
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
  }

  # ── cloud-init seed ISO bundled into artifact ────────────────────────────
  post-processor "shell-local" {
    inline = [
      "echo 'instance-id: shinyfarm' > meta-data",
      "cloud-localds --network-config=network-config.yml user-seed.iso user-data.yaml meta-data"
    ]
  }
}