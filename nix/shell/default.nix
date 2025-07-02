pkgs:
pkgs.mkShell {
  name = "sift";
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
    cargo-nextest
    gnuplot
    kani
  ];
}
