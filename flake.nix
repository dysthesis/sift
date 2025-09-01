{
  description = "sift - a feed reader and archival utility that learns";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

    crane.url = "github:ipetkov/crane";

    flake-parts.url = "github:hercules-ci/flake-parts";

    nur = {
      url = "github:nix-community/NUR";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-parts.follows = "flake-parts";
    };

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    advisory-db = {
      url = "github:rustsec/advisory-db";
      flake = false;
    };
  };

  outputs = inputs @ {
    crane,
    flake-parts,
    advisory-db,
    ...
  }:
    flake-parts.lib.mkFlake {inherit inputs;} (
      _: {
        flake = {
          # Place additional top-level outputs here if needed
        };

        systems = [
          "x86_64-linux"
          "aarch64-linux"
          "x86_64-darwin"
          "aarch64-darwin"
        ];

        perSystem = {
          config,
          pkgs,
          lib,
          system,
          ...
        }: let
          craneLib = crane.mkLib pkgs;
          src = craneLib.cleanCargoSource ./.;

          # Common arguments can be set here to avoid repeating them later
          commonArgs = {
            inherit src;
            strictDeps = true;

            buildInputs =
              lib.optionals pkgs.stdenv.isLinux [
                pkgs.openssl
              ]
              ++ lib.optionals pkgs.stdenv.isDarwin [
                # Additional darwin specific inputs can be set here
                pkgs.libiconv
              ];

            nativeBuildInputs = lib.optionals pkgs.stdenv.isLinux [pkgs.pkg-config];

            # Additional environment variables can be set directly
            # MY_CUSTOM_VAR = "some value";
          };

          # Build *just* the cargo dependencies (of the entire workspace),
          # so we can reuse all of that work (e.g. via cachix) when running in CI
          # It is *highly* recommended to use something like cargo-hakari to avoid
          # cache misses when building individual top-level-crates
          cargoArtifacts = craneLib.buildDepsOnly commonArgs;

          individualCrateArgs =
            commonArgs
            // {
              inherit cargoArtifacts;
              inherit (craneLib.crateNameFromCargoToml {inherit src;}) version;
              # NB: we disable tests since we'll run them all via cargo-nextest
              doCheck = false;
            };

          fileSetForCrate = crate:
            lib.fileset.toSource {
              root = ./.;
              fileset = lib.fileset.unions [
                ./Cargo.toml
                ./Cargo.lock
                (craneLib.fileset.commonCargoSources ./crates/siftd)
                (craneLib.fileset.commonCargoSources crate)
              ];
            };

          # Build the top-level crates of the workspace as individual derivations.
          # This allows consumers to only depend on (and build) only what they need.
          # Though it is possible to build the entire workspace as a single derivation,
          # so this is left up to you on how to organize things
          #
          # Note that the cargo workspace must define `workspace.members` using wildcards,
          # otherwise, omitting a crate (like we do below) will result in errors since
          # cargo won't be able to find the sources for all members.
          siftd = craneLib.buildPackage (
            individualCrateArgs
            // {
              pname = "siftd";
              cargoExtraArgs = "-p siftd";
              src = fileSetForCrate ./crates/siftd;
            }
          );
        in {
          _module.args.pkgs = import inputs.nixpkgs {
            inherit system;
            overlays = [
              inputs.rust-overlay.overlays.default
              inputs.nur.overlays.default
            ];
          };
          checks = {
            # Build the crates as part of `nix flake check` for convenience
            inherit siftd;

            # Run clippy (and deny all warnings) on the workspace source,
            # again, reusing the dependency artifacts from above.
            #
            # Note that this is done as a separate derivation so that
            # we can block the CI if there are issues here, but not
            # prevent downstream consumers from building our crate by itself.
            my-workspace-clippy = craneLib.cargoClippy (
              commonArgs
              // {
                inherit cargoArtifacts;
                cargoClippyExtraArgs = "--all-targets -- --deny warnings";
              }
            );

            my-workspace-doc = craneLib.cargoDoc (
              commonArgs
              // {
                inherit cargoArtifacts;
                # This can be commented out or tweaked as necessary, e.g. set to
                # `--deny rustdoc::broken-intra-doc-links` to only enforce that lint
                env.RUSTDOCFLAGS = "--deny warnings";
              }
            );

            # Check formatting
            my-workspace-fmt = craneLib.cargoFmt {
              inherit src;
            };

            my-workspace-toml-fmt = craneLib.taploFmt {
              src = pkgs.lib.sources.sourceFilesBySuffices src [".toml"];
              # taplo arguments can be further customized below as needed
              # taploExtraArgs = "--config ./taplo.toml";
            };

            # Audit dependencies
            my-workspace-audit = craneLib.cargoAudit {
              inherit src advisory-db;
            };

            # Audit licenses
            my-workspace-deny = craneLib.cargoDeny {
              inherit src;
            };

            # Run tests with cargo-nextest
            # Consider setting `doCheck = false` on other crate derivations
            # if you do not want the tests to run twice
            my-workspace-nextest = craneLib.cargoNextest (
              commonArgs
              // {
                inherit cargoArtifacts;
                partitions = 1;
                partitionType = "count";
                cargoNextestPartitionsExtraArgs = "--no-tests=pass";
              }
            );

            # Ensure that cargo-hakari is up to date
            my-workspace-hakari = craneLib.mkCargoDerivation {
              inherit src;
              pname = "my-workspace-hakari";
              cargoArtifacts = null;
              doInstallCargoArtifacts = false;

              buildPhaseCargoCommand = ''
                cargo hakari generate --diff  # workspace-hack Cargo.toml is up-to-date
                cargo hakari manage-deps --dry-run  # all workspace crates depend on workspace-hack
                cargo hakari verify
              '';

              nativeBuildInputs = [
                pkgs.cargo-hakari
              ];
            };
          };

          packages = {
            inherit siftd;
          };

          apps = {
            siftd = {
              type = "app";
              program = "${siftd}/bin/siftd";
            };
          };

          devShells.default = craneLib.devShell {
            # Inherit inputs from checks for tools and env.
            inherit (config) checks;

            # Additional dev-shell environment variables can be set directly
            # MY_CUSTOM_DEVELOPMENT_VAR = "something else";

            # Extra inputs can be added here; cargo and rustc are provided by default.
            packages = let
              # Kani - Rust model checker
              kani = pkgs.callPackage ./nix/pkgs/kani {
                inherit (inputs) rust-overlay;
              };
            in
              with pkgs; [
                nixd
                statix
                deadnix
                nixfmt
                alejandra

                cargo-audit
                cargo-expand
                cargo-nextest
                bacon
                rust-analyzer
                kani

                atac
              ];
          };
        };
      }
    );
}
