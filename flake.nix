{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    systems.url = "github:nix-systems/default";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs =
    {
      nixpkgs,
      systems,
      rust-overlay,
      ...
    }:
    let
      eachSystem = nixpkgs.lib.genAttrs (import systems);
    in
    {
      devShells = eachSystem (
        system:
        let
          pkgs = import nixpkgs {
            inherit system;
            overlays = [ rust-overlay.overlays.default ];
          };

          rustToolchain = pkgs.rust-bin.nightly.latest.default.override {
            extensions = [
              "rust-src"
              "rust-analyzer"
              "rustc-codegen-cranelift-preview"
            ];
          };
        in
        {
          default = (pkgs.mkShell.override { stdenv = pkgs.clangStdenv; }) {
            packages = with pkgs; [
              xorg.libX11
              xorg.libXrandr
              xorg.libXinerama
              xorg.libXcursor
              xorg.libXi

              vulkan-loader
              vulkan-validation-layers

              shaderc
              cmake
              mold-wrapped
              rustToolchain
              rustPlatform.bindgenHook
            ];
          };
        }
      );
    };
}
