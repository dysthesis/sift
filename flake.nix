{
  description = "CLI RSS feed reader";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs";
    kani-repo = {
      url = "git+https://github.com/model-checking/kani?ref=main&rev=0182e99acdfff89f3f55b7324823d8d7c540a959&submodules=1";
      flake = false;
    };
    kani-tarball = {
      url = "https://github.com/model-checking/kani/releases/download/kani-0.56.0/kani-0.56.0-x86_64-unknown-linux-gnu.tar.gz";
      flake = false;
    };
    treefmt-nix = {
      url = "github:numtide/treefmt-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    # Personal library
    babel = {
      url = "github:dysthesis/babel";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };
  outputs = inputs @ {
    self,
    babel,
    nixpkgs,
    treefmt-nix,
    rust-overlay,
    kani-tarball,
    kani-repo,
    ...
  }: let
    inherit (builtins) mapAttrs;
    inherit (babel) mkLib;
    lib = mkLib nixpkgs;

    # Systems to support
    systems = [
      "aarch64-linux"
      "x86_64-linux"
      "x86_64-darwin"
      "aarch64-darwin"
    ];

    overlays = [
      rust-overlay.overlays.default
      (final: _prev: {
        rustToolchains = {
          stable = final.rust-bin.stable.latest.default.override {
            extensions = [
              "rust-src"
              "rust-analyzer"
              "llvm-tools"
            ];
          };
          nightly = final.rust-bin.nightly.latest.default.override {
            extensions = [
              "rust-src"
              "rust-analyzer"
              "miri"
              "rustc-dev"
              "llvm-tools-preview"
            ];
          };
        };
        kani = let
          rustHome = final.rust-bin.nightly."2024-10-03".default.override {
            extensions = ["rustc-dev" "rust-src" "llvm-tools" "rustfmt"];
          };
          rustPlatform = final.makeRustPlatform {
            cargo = rustHome;
            rustc = rustHome;
          };

          kani-home = final.stdenv.mkDerivation {
            name = "kani-home";

            src = kani-tarball;

            buildInputs = [
              final.stdenv.cc.cc.lib #libs needed by patchelf
            ];

            runtimeDependencies = [
              final.glibc #not detected as missing by patchelf for some reason
            ];

            nativeBuildInputs = [final.autoPatchelfHook];

            installPhase = ''
              runHook preInstall
              ${final.rsync}/bin/rsync -av $src/ $out --exclude kani-compiler
              runHook postInstall
            '';
          };

          kani = rustPlatform.buildRustPackage rec {
            pname = "kani";

            version = "kani-0.56.0";

            src = kani-repo;

            nativeBuildInputs = [final.makeWrapper];

            postInstall = ''
              mkdir -p $out/lib/
              ${final.rsync}/bin/rsync -av ${kani-home}/ $out/lib/${version} --perms --chmod=D+rw,F+rw
              cp $out/bin/* $out/lib/${version}/bin/
              ln -s ${rustHome} $out/lib/${version}/toolchain
            '';

            postFixup = ''
              wrapProgram $out/bin/kani --set KANI_HOME $out/lib/
              wrapProgram $out/bin/cargo-kani --set KANI_HOME $out/lib/
            '';

            cargoHash = "sha256-b2FfHxSxnfmd4eZfmRAbKVNyBd3Q4/ndJKx1b3WDUiA=";

            env = {
              RUSTUP_HOME = "${rustHome}";
              RUSTUP_TOOLCHAIN = "..";
            };
          };
        in
          if final.system == "x86_64-linux"
          then kani
          else throw "Oops! ${final.system} not supported by this kani derivation";
      })
    ];

    forAllSystems = lib.babel.forAllSystems {inherit systems overlays;};

    treefmt = forAllSystems (pkgs: treefmt-nix.lib.evalModule pkgs ./nix/formatters);
  in
    # Budget flake-parts
    mapAttrs (_: forAllSystems) {
      devShells = pkgs: {default = import ./nix/shell pkgs;};
      # for `nix fmt`
      formatter = pkgs: treefmt.${pkgs.system}.config.build.wrapper;
      # for `nix flake check`
      checks = pkgs: {
        formatting = treefmt.${pkgs.system}.config.build.check self;
      };
      packages = pkgs:
        import ./nix/packages {
          inherit
            self
            pkgs
            inputs
            lib
            ;
        };
    };
}
