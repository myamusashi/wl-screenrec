{
  description = "wl-screenrec - High performance screen/audio recorder for wlroots";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
      in
      {
        packages.default = pkgs.callPackage ./nix/package.nix {
          src = pkgs.lib.cleanSource ./.;
        };
        packages.wl-screenrec = pkgs.callPackage ./nix/package.nix {
          src = pkgs.lib.cleanSource ./.;
        };

        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            rustc
            cargo
            clippy
            rustfmt
            pkg-config
            libclang
            clang
            ffmpeg
            wayland
            libdrm
          ];
          LIBCLANG_PATH = "${pkgs.libclang.lib}/lib";
          BINDGEN_EXTRA_CLANG_ARGS = "-isystem ${pkgs.glibc.dev}/include";
        };
      });
}
