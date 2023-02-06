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
  };

  outputs = { self, nixpkgs, devenv, fenix, flake-utils, crane, ... } @ inputs:
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
            inherit cargoArtifacts ;
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
          packages.default = tinyvmm;
          checks = {
            inherit tinyvmm tinyvmmClippy tinyvmmCoverage;
            integration = pkgs.nixosTest (import ./checks/integration/bridge.nix { nixosModule = self.nixosModules.default; });
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
                type = with types; bool;
              };
            };
          };
          config = mkIf cfg.enable {
            systemd.services.tinyvmm = {
              wantedBy = [ "multi-user.target" ];
              script = "${self.packages.${pkgs.system}.default}/bin/tinyvmm serve --reconcile-delay 10 --listen \${RUNTIME_DIRECTORY}/sock -vvvv";
              serviceConfig = {
                RuntimeDirectory = [ "tinyvmm" ];
                StateDirectory = [ "tinyvmm" ]; # TODO: expose via a flag
              };
            };
          };
        };
    };
}
