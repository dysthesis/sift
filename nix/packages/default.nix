{
  self,
  pkgs,
  lib,
  inputs,
  ...
}:
rec {
  default = sift;
  sift = pkgs.callPackage ./sift.nix {
    inherit
      pkgs
      inputs
      lib
      self
      ;
  };
}
