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
        ];

        # Enable debug verbosity.
        boot.consoleLogLevel = 7;

        # Convenience packages for interactive use
        environment.systemPackages = [ pkgs.pciutils pkgs.usbutils ];

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
in
{
  integration-smoke = pkgs.nixosTest {
    name = "usbvfiod Smoke Test";

    nodes.machine = { pkgs, ... }: {
      boot.kernelModules = [ "kvm" ];
      systemd.services.usbvfiod = {
        wantedBy = [ "multi-user.target" ];

        serviceConfig = {
          ExecStart = ''
            ${lib.getExe usbvfiod} -v --socket-path ${usbvfiodSocket}
          '';
        };
      };

      systemd.services.cloud-hypervisor = {
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

      virtualisation = {
        cores = 2;
        memorySize = 4096;
      };
    };

    # The nested CI runs are really slow.
    globalTimeout = 3600;
    testScript = ''
      start_all()

      machine.wait_for_unit("cloud-hypervisor.service")

      # Check whether the USB controller pops up.
      machine.wait_until_succeeds("grep -Fq 'usb usb1: Product: xHCI Host Controller' ${cloudHypervisorLog}", timeout=3000)
      machine.wait_until_succeeds("grep -Fq 'hub 1-0:1.0: 1 port detected' ${cloudHypervisorLog}")
    '';
  };
}
