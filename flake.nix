{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-22.11";
    fenix.url = "github:nix-community/fenix";
    fenix.inputs.nixpkgs.follows = "nixpkgs";
    devenv.url = "github:cachix/devenv";
    flake-utils.url = "github:numtide/flake-utils";
    crane = {
      url = "github:ipetkov/crane";
      inputs = {
        flake-utils.follows = "flake-utils";
        nixpkgs.follows = "nixpkgs";
      };
    };
    nixos-generators = {
      url = "github:nix-community/nixos-generators";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    { self
    , nixpkgs
    , devenv
    , fenix
    , flake-utils
    , crane
    , nixos-generators
    , ...
    } @ inputs:
    flake-utils.lib.eachSystem [ flake-utils.lib.system.x86_64-linux ]
      (system:
      let
        pkgs = import nixpkgs { inherit system; };
        rustVersion = "stable";
        fenixPkgs = fenix.packages.${system}.${rustVersion};
        toolchain = fenixPkgs.toolchain;
        craneLib = crane.lib.${system}.overrideToolchain toolchain;
        runtimeInputs = with pkgs; [
          cloud-hypervisor
          nftables
        ];
        buildInputs = with pkgs; [
          mold
          clang
        ];
        commonArgs = {
          inherit buildInputs;
          src = craneLib.cleanCargoSource ./.;
          pname = "tinyvmm";
        };
        cargoArtifacts = craneLib.buildDepsOnly (commonArgs // {
          pname = "tinyvmm-deps";
        });
        tinyvmmClippy = craneLib.cargoClippy (commonArgs // {
          inherit cargoArtifacts;
          cargoClippyExtraArgs = "--all-targets -- --deny warnings";
        });
        tinyvmmCoverage = craneLib.cargoTarpaulin (commonArgs // {
          inherit cargoArtifacts;
        });
        tinyvmm = craneLib.buildPackage (commonArgs // {
          inherit cargoArtifacts;
        });
      in
      {
        packages = {
          default = tinyvmm;
          hypervisor-firmware = pkgs.fetchurl {
            name = "hypervisor-firmware";
            url = "https://github.com/cloud-hypervisor/rust-hypervisor-firmware/releases/download/0.4.2/hypervisor-fw";
            sha256 = "sha256-WMFGE7xmBnI/GBJNAPujRk+vMx1ssGp//lbeYtgHEkA=";
          };
          test-vm = nixos-generators.nixosGenerate {
            system = "x86_64-linux";
            modules = [
              ({ config, ... }: {
                system.stateVersion = "22.11";

                networking.usePredictableInterfaceNames = false;
                networking.interfaces.eth0.useDHCP = false;
                networking.interfaces.eth0.ipv4.addresses = [{
                  address = "10.0.0.2";
                  prefixLength = 24;
                }];
                networking.defaultGateway = "10.0.0.1";
                networking.nameservers = [ "8.8.8.8" ];

                boot = {
                  kernelParams = [ "console=ttyS0" "panic=1" "boot.panic_on_fail" ];
                  initrd.kernelModules = [ "virtio_scsi" "virtio_pci" "virtio_net" ];
                  kernelModules = [ "virtio_pci" "virtio_net" ];
                };
              })
            ];
            format = "raw-efi";
          };
        };
        checks = {
          inherit tinyvmm tinyvmmClippy tinyvmmCoverage;
          integration.bridge = pkgs.nixosTest (import ./checks/integration/bridge.nix self);
          integration.ch = pkgs.nixosTest (import ./checks/integration/ch.nix self);
        };
        devShells.default = devenv.lib.mkShell {
          inherit inputs pkgs;
          modules = [
            (
              {
                languages.rust.enable = true;
                languages.rust.version = rustVersion;
                packages = with pkgs; [
                  sqlite
                  rustfmt
                  cargo-whatfeatures
                  cargo-watch
                  fenixPkgs.clippy
                ] ++ runtimeInputs ++ buildInputs;
              }
            )
          ];
        };
      }
      ) // {
      nixosModules.default = { config, pkgs, lib, ... }:
        let
          cfg = config.services.tinyvmm;
        in
        with lib;
        {
          options = {
            services.tinyvmm = {
              enable = mkOption {
                default = false;
                type = with types; uniq bool;
              };
              dnsAddress = mkOption {
                default = "127.0.0.2:53";
                type = with types; uniq string;
              };
            };
          };
          config = mkIf cfg.enable {
            systemd.services.tinyvmm = {
              wantedBy = [ "multi-user.target" ];
              script = ''
                ${self.packages.${pkgs.system}.default}/bin/tinyvmm \
                --store ''${STATE_DIRECTORY}/store.db \
                serve \
                --listen ''${RUNTIME_DIRECTORY}/sock \
                --listen-dns "${cfg.dnsAddress}" \
                -vvv
              '';
              serviceConfig = {
                RuntimeDirectory = [ "tinyvmm" ];
                StateDirectory = [ "tinyvmm" ];
              };
            };
          };
        };
    };
}
