pkgs:
pkgs.mkShell {
  name = "sim";
  packages = with pkgs; [
    nixd
    alejandra
    statix
    deadnix
    npins
    cargo
    rustToolchains.nightly
    bacon
    cargo-flamegraph
    gnuplot
  ];
}
