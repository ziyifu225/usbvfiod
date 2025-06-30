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

        # Convenience packages for interactive use
        environment.systemPackages = with pkgs; [ pciutils usbutils ];

        # Add user services that run on automatic login.
        systemd.user.services = {
          dump-diagnostics = {
            description = "Dump diagnostic information";
            wantedBy = [ "default.target" ];

            serviceConfig = {
              ExecStart = pkgs.writeShellScript "diagnostic-dump" ''
                echo Dumping Diagnostics
                cat /proc/interrupts
              '';
              StandardOutput = "journal+console";
              StandardError = "journal+console";
            };
          };
        };

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
  usbDiskImage = pkgs.writeText "usb-stick-image.raw" ''
    This is an uninitialized drive.
  '';

  # Report for target device path and access permissions
  usbDeviceInfoScript = pkgs.writeShellScript "usb-device-info" ''
    echo "----- USB Block Device (/dev/sdX) -----"
    usb_device=$(lsblk -SJ | jq -r '.blockdevices[] | select(.tran == "usb") | .name')
    dev="/dev/$usb_device"
    echo "Device path: $dev"
    echo "--- USB Block Device (/dev/sdX) Permissions ---"
    ls -l $dev || echo "Could not stat block device $dev"

    echo "----- USB Bus-Exposed Device -----"
    vendor=$(udevadm info --query=all --name=$dev | grep -oP 'ID_USB_VENDOR_ID=\K\w+' || true)
    model=$(udevadm info --query=all --name=$dev | grep -oP 'ID_USB_MODEL_ID=\K\w+' || true)
    bus_usb_device=$(lsusb -d "$vendor:$model" || true)
    echo "$bus_usb_device"
    bus=$(awk '{print $2}' <<<"$bus_usb_device")
    dev_num=$(awk '{ gsub(":",""); print $4 }' <<<"$bus_usb_device")
    path="/dev/bus/usb/$bus/$dev_num"
    echo "Character device path: $path"
    ls -l "$path" || echo "Could not stat character device $path"

    exit 0
  '';

  # detect USB device with Symlink
  detectUsbScript = pkgs.writeShellScript "detect-usb" ''
    set -euxo pipefail
    for i in {1..10}; do
      [ -L /dev/teststorage ] && break
      sleep 0.5
    done

    [ -L /dev/teststorage ] || {
      echo "Symlink /dev/teststorage not found" >&2
      exit 0
    }

    resolved=$(readlink -f /dev/teststorage)
    echo "Found USB device at $resolved"
    echo "USBVFIOD_DEVICE=$resolved" > /run/usbvfiod.env
  '';
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
        ACTION=="add", SUBSYSTEM=="usb", ATTRS{idVendor}=="${vendorId}", ATTRS{idProduct}=="${productId}", SYMLINK+="bus/usb/teststorage"
      '';

      users.users.testUser = {
        isNormalUser = true;
        extraGroups = [ ];
        password = "test";
      };

      boot.kernelModules = [ "kvm" ];
      systemd.services = {
        detect-usb-device = {
          wantedBy = [ "multi-user.target" ];
          before = [ "usbvfiod.service" ];

          serviceConfig = {
            Type = "oneshot";
            RemainAfterExit = true;
            ExecStart = detectUsbScript;
          };
        };

        usbvfiod = {
          wantedBy = [ "multi-user.target" ];
          after = [ "detect-usb-device.service" ];
          requires = [ "detect-usb-device.service" ];

          serviceConfig = {
            # User = "testUser";
            # Group = "users";
            # EnvironmentFile = "/run/usbvfiod.env";         
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
            ExecStart = ''
              ${lib.getExe pkgs.cloud-hypervisor} --memory size=2G,shared=on --console off --serial file=${cloudHypervisorLog} \
                --kernel ${netboot.kernel} \
                --cmdline ${lib.escapeShellArg netboot.cmdline} \
                --initramfs ${netboot.initrd} \
                --user-device socket=${usbvfiodSocket}
            '';
          };
        };
      };

      virtualisation = {
        cores = 2;
        memorySize = 4096;
        qemu.options = [
          # A virtual USB XHCI controller in the host ...
          "-device qemu-xhci,id=host-xhci,addr=10"
          # ... with an attached usb stick.
          "-drive if=none,id=usbstick,format=raw,snapshot=on,file=${usbDiskImage}"
          "-device usb-storage,bus=host-xhci.0,drive=usbstick"
        ];
      };
    };

    # The nested CI runs are really slow.
    globalTimeout = 3600;
    testScript = ''
      start_all()

      # Display device path and access permissions
      print("-------- USB Device Information Report --------")
      stdout = machine.execute("${usbDeviceInfoScript}")[1]
      print(stdout)

      machine.wait_for_unit("cloud-hypervisor.service")

      # Check whether the USB controller pops up.
      machine.wait_until_succeeds("grep -Fq 'usb usb1: Product: xHCI Host Controller' ${cloudHypervisorLog}", timeout=3000)
      machine.wait_until_succeeds("grep -Fq 'hub 1-0:1.0: 1 port detected' ${cloudHypervisorLog}")

      # Read the diagnostic information after login.
      machine.wait_until_succeeds("grep -Eq '\s+1\s+PCI-MSIX.*xhci_hcd' ${cloudHypervisorLog}")
    '';
  };
}
