{
  description = "The claude-code-transcript development shell, every tool the gate runs, pinned by the lock.";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    { nixpkgs, rust-overlay, ... }:
    let
      systems = [
        "aarch64-darwin"
        "x86_64-darwin"
        "aarch64-linux"
        "x86_64-linux"
      ];
      shellFor =
        system:
        let
          pkgs = import nixpkgs {
            inherit system;
            overlays = [ rust-overlay.overlays.default ];
          };
        in
        pkgs.mkShell {
          # Set inside this shell alone, so a recipe can tell it already stands in it
          # and need not enter it again.
          CLAUDE_CODE_TRANSCRIPT_SHELL = "1";
          # The toolchain is the one `rust-toolchain.toml` pins, so CI's rustup and this
          # shell resolve the same compiler from one file.
          packages = [
            (pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml)
            pkgs.just
          ];
        };
    in
    {
      devShells = nixpkgs.lib.genAttrs systems (system: {
        default = shellFor system;
      });
    };
}
