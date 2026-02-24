{ pkgs ? import <nixpkgs> {}}:
pkgs.mkShell {
  packages = with pkgs; [
    cargo
    tree-sitter
    nodejs
  ];
}
