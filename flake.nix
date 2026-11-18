{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-26.05";

    flake-parts = {
      url = "github:hercules-ci/flake-parts";
      inputs.nixpkgs-lib.follows = "nixpkgs";
    };

    x52 = {
      url = "github:x52dev/nix";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-parts.follows = "flake-parts";
    };
  };

  outputs = inputs@{ flake-parts, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      imports = [ inputs.x52.flakeModules.default ];

      systems = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];

      perSystem = { pkgs, config, inputs', ... }: {
        formatter = pkgs.nixpkgs-fmt;

        devShells.ci-release = pkgs.mkShellNoCC {
          packages = [ inputs'.x52.packages.x52-release-tools ];
        };

        devShells.default = pkgs.mkShell {
          packages = [
            config.formatter
            pkgs.cargo-deny
            pkgs.cargo-hack
            pkgs.cargo-rdme
            pkgs.cmake
            pkgs.fd
            pkgs.jq
            pkgs.just
            pkgs.nats-server
            pkgs.taplo
          ];

          shellHook = config.x52.justRust.shellHook;
        };
      };
    };
}
