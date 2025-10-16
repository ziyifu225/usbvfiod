# This file contains integration tests for usbvfiod.
{ lib, pkgs, usbvfiod }:
let
  # For the VM that we start in Cloud Hypervisor, we re-use the netboot image.
  netbootNixos = lib.nixosSystem {
    inherit (pkgs) system;

    modules = [
      "${pkgs.path}/nixos/modules/installer/netboot/netboot-minimal.nix"

      # Cloud Hypervisor Guest Convenience
      ({ config, ... }: {

        boot.kernelParams = [
          # Use the serial console for kernel output.
          #
          # The virtio-console is an option as well, but is not
          # compiled into the NixOS kernel and would be inconvenient.
          "console=ttyS0"
          # Enable dyndbg messages for the XHCI driver.
          "xhci_pci.dyndbg==pmfl"
          "xhci_hcd.dyndbg==pmfl"
        ];

        # Enable debug verbosity.
        boot.consoleLogLevel = 8;

        # allow disk access for users
        users.users.nixos.extraGroups = [ "disk" ];

        # Convenience packages for interactive use
        environment.systemPackages = with pkgs; [ pciutils usbutils ];

        # Add user services that run on automatic login.
        systemd.user.services = {
          diagnostic-tests = {
            description = "Run diagnostic tests";
            wantedBy = [ "default.target" ];

            serviceConfig = {
              ExecStart = pkgs.writeShellScript "diagnostic-tests" ''
                echo Running Diagnostics
                cat /proc/interrupts
                echo " "
                ${pkgs.usbutils}/bin/lsusb
                ${pkgs.util-linux}/bin/sfdisk -l
                echo " "
                echo ',,L' | /run/wrappers/bin/sudo ${pkgs.util-linux}/bin/sfdisk --label=gpt /dev/sda
                echo " "
                /run/wrappers/bin/sudo ${pkgs.e2fsprogs}/bin/mkfs.ext4 /dev/sda1 && echo "Successfully created a new ext4 filesystem on the blockdevice."
                echo " "
                /run/wrappers/bin/sudo ${pkgs.coreutils}/bin/mkdir -p /mnt
                /run/wrappers/bin/sudo ${pkgs.util-linux}/bin/mount -o X-mount.owner=nixos /dev/sda1 /mnt
                echo " "
                echo "This is a new partition with ext4 filesystem." > /mnt/file.txt
                /run/wrappers/bin/sudo ${pkgs.util-linux}/bin/umount /mnt
                /run/wrappers/bin/sudo ${pkgs.util-linux}/bin/mount -o X-mount.owner=nixos /dev/sda1 /mnt
                cat /mnt/file.txt
              '';
              StandardOutput = "journal+console";
              StandardError = "journal+console";
            };
          };
        };

        # network configuration for interactive debugging
        networking.interfaces."ens1" = {
          ipv4.addresses = [
            {
              address = "192.168.100.2";
              prefixLength = 24;
            }
          ];
          ipv4.routes = [
            {
              address = "0.0.0.0";
              prefixLength = 0;
              via = "192.168.100.1";
            }
          ];
          useDHCP = false;
        };

        # ssh access for interactive debugging
        services.openssh = {
          enable = true;
          settings = {
            PermitRootLogin = "yes";
            PermitEmptyPasswords = "yes";
          };
        };
        security.pam.services.sshd.allowNullPassword = true;

        # Silence the useless stateVersion warning. We have no state to keep.
        system.stateVersion = config.system.nixos.release;
      })
    ];
  };

  netboot =
    let
      inherit (netbootNixos) config;

      kernelTarget = pkgs.stdenv.hostPlatform.linux-kernel.target;
    in
    {
      initrd = "${config.system.build.netbootRamdisk}/initrd";
      kernel = "${config.system.build.kernel}/${kernelTarget}";
      cmdline = "init=${config.system.build.toplevel}/init "
        + builtins.toString config.boot.kernelParams;
    };

  # Putting the socket in a world-readable location is obviously not a
  # good choice for a production setup, but for this test it works
  # well.
  usbvfiodSocket = "/tmp/usbvfio";
  cloudHypervisorLog = "/tmp/chv.log";
  vendorId = "46f4";
  productId = "0001";

  # Provide a raw file as usb stick test image.
  blockDeviceFile = "/tmp/image.img";
  blockDeviceSize = "8M";

in
{
  integration-smoke = pkgs.nixosTest {
    name = "usbvfiod Smoke Test";

    nodes.machine = { pkgs, ... }: {
      environment.systemPackages = with pkgs; [
        jq
        usbutils
      ];

      services.udev.extraRules = ''
        ACTION=="add", SUBSYSTEM=="usb", ATTRS{idVendor}=="${vendorId}", ATTRS{idProduct}=="${productId}", MODE="0660", GROUP="usbaccess", SYMLINK+="bus/usb/teststorage"
      '';

      users.groups.usbaccess = { };

      users.users.usbaccess = {
        isSystemUser = true;
        group = "usbaccess";
      };

      boot.kernelModules = [ "kvm" ];
      systemd.services = {
        usbvfiod = {
          wantedBy = [ "multi-user.target" ];

          serviceConfig = {
            User = "usbaccess";
            Group = "usbaccess";
            ExecStart = ''
              ${lib.getExe usbvfiod} -v --socket-path ${usbvfiodSocket} --device "/dev/bus/usb/teststorage"
            '';
          };
        };

        cloud-hypervisor = {
          wantedBy = [ "multi-user.target" ];
          requires = [ "usbvfiod.service" ];
          after = [ "usbvfiod.service" ];

          serviceConfig = {
            Restart = "on-failure";
            RestartSec = "2s";
            ExecStart = ''
              ${lib.getExe pkgs.cloud-hypervisor} --memory size=2G,shared=on --console off --serial file=${cloudHypervisorLog} \
                --kernel ${netboot.kernel} \
                --cmdline ${lib.escapeShellArg netboot.cmdline} \
                --initramfs ${netboot.initrd} \
                --user-device socket=${usbvfiodSocket} \
                --net "tap=tap0,mac=,ip=192.168.100.1,mask=255.255.255.0"
            '';
          };
        };
      };

      # interactive debugging
      services.openssh = {
        enable = true;
        settings = {
          PermitRootLogin = "yes";
          PermitEmptyPasswords = "yes";
        };
      };
      security.pam.services.sshd.allowNullPassword = true;
      virtualisation.forwardPorts = [
        { from = "host"; host.port = 2000; guest.port = 22; }
      ];

      virtualisation = {
        cores = 2;
        memorySize = 4096;
        qemu.options = [
          # A virtual USB XHCI controller in the host ...
          "-device qemu-xhci,id=host-xhci,addr=10"
          # ... with an attached usb stick.
          "-drive if=none,id=usbstick,format=raw,file=${blockDeviceFile}"
          "-device usb-storage,bus=host-xhci.0,drive=usbstick"
        ];
      };
    };

    # The nested CI runs are really slow.
    globalTimeout = 3600;
    testScript = ''
      import os
      print("Creating file image at ${blockDeviceFile}")
      os.system("rm ${blockDeviceFile}")
      os.system("dd bs=1  count=1 seek=${blockDeviceSize} if=/dev/zero of=${blockDeviceFile}")

      start_all()

      machine.wait_for_unit("cloud-hypervisor.service")

      # Check whether the USB controller pops up.
      machine.wait_until_succeeds("grep -Fq 'usb usb1: Product: xHCI Host Controller' ${cloudHypervisorLog}", timeout=3000)
      machine.wait_until_succeeds("grep -Eq 'hub 1-0:1\\.0: [0-9]+ ports? detected' ${cloudHypervisorLog}")

      # Read the diagnostic information after login.
      machine.wait_until_succeeds("grep -Eq '\s+[1-9][0-9]*\s+PCI-MSIX.*xhci_hcd' ${cloudHypervisorLog}")
      machine.wait_until_succeeds("grep -q 'ID ${vendorId}:${productId} QEMU QEMU USB HARDDRIVE' ${cloudHypervisorLog}")
      machine.wait_until_succeeds("grep -q 'Disk /dev/sda:' ${cloudHypervisorLog}")

      # Confirm the partition creation was successful.
      machine.wait_until_succeeds("grep -q 'Disklabel type: gpt' ${cloudHypervisorLog}")
      machine.wait_until_succeeds("grep -Eq '/dev/sda1 .* Linux filesystem' ${cloudHypervisorLog}")

      # Confirm the filesystem is functional.
      machine.wait_until_succeeds("grep -q 'Successfully created a new ext4 filesystem on the blockdevice.' ${cloudHypervisorLog}")
      machine.wait_until_succeeds("grep -q 'This is a new partition with ext4 filesystem.' ${cloudHypervisorLog}")
    '';
  };
}
