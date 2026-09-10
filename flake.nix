{
  description = "A Wayland wallpaper daemon with animated transitions";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay/stable";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
      flake-utils,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlays.default ];
        };
        toolchain = pkgs.rust-bin.stable.latest.default;
        rustPlatform = pkgs.makeRustPlatform {
          cargo = toolchain;
          rustc = toolchain;
        };
        fmtDate =
          raw:
          let
            year = builtins.substring 0 4 raw;
            month = builtins.substring 4 2 raw;
            day = builtins.substring 6 2 raw;
          in
          "${year}-${month}-${day}";
        rev = self.rev or "dirty";
        date = fmtDate self.lastModifiedDate;
        version = "unstable-${date}-${self.shortRev or "dirty"}";
      in
      {
        packages = {
          ouranos = pkgs.callPackage ./nix/package.nix {
            inherit
              date
              rev
              rustPlatform
              version
              ;
          };
          default = self.packages.${system}.ouranos;
        };

        devShells = {
          default = pkgs.callPackage ./nix/shell.nix {
            inherit (self.packages.${system}) ouranos;
          };
        };

        formatter = pkgs.nixfmt-tree;
      }
    )
    // {
      overlays.default = _: prev: {
        inherit (self.packages.${prev.stdenv.system}) ouranos;
      };

      homeManagerModules.default = { lib, pkgs, ... }: {
        imports = [ ./nix/home-module.nix ];
        services.ouranos.package = lib.mkDefault self.packages.${pkgs.stdenv.hostPlatform.system}.default;
      };
    };
}
