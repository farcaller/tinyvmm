{ nixosModule }: {
  name = "bridge";

  nodes.vmm = { config, pkgs, ... }: {
    imports = [ nixosModule ];
    systemd.network.enable = true;

    environment.systemPackages = with pkgs; [
      curl
      iproute2
      jq
      sqlite
    ];
    services.tinyvmm.enable = true;
  };

  testScript = builtins.readFile ./bridge.py;
}
