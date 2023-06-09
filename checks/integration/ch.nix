self: {
  name = "ch";

  nodes.vmm = { config, pkgs, ... }: {
    systemd.network.enable = true;

    environment.systemPackages = with pkgs; [
      curl
      iproute2
      jq
      sqlite
      # self.packages.x86_64-linux.test-vm
      # self.packages.x86_64-linux.hypervisor-firmware
    ];

    security.wrappers.cloud-hypervisor = {
      owner = "root";
      group = "root";
      capabilities = "cap_net_admin+ep";
      source = "${pkgs.cloud-hypervisor}/bin/cloud-hypervisor";
    };

    systemd.services.test-vm = {
      wantedBy = [ "multi-user.target" ];
      # cat ${self.packages.x86_64-linux.test-vm}/nixos.img > /tmp/image          
      script = ''
        /run/wrappers/bin/cloud-hypervisor \
          --kernel ${self.packages.x86_64-linux.hypervisor-firmware} \
          --disk path=${self.packages.x86_64-linux.test-vm}/nixos.img,readonly=on \
          --cpus boot=1 \
          --memory size=512M \
          --net "tap=,mac=,ip=10.0.0.1,mask=255.255.255.0" \
          --console off \
          --serial tty
      '';
    };
  };

  testScript = builtins.readFile ./ch.py;
}
