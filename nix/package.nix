{
  lib,
  stdenv,
  rustPlatform,
  pkg-config,
  installShellFiles,
  libdrm,
  ffmpeg,
  wayland,
  src,
}:

rustPlatform.buildRustPackage {
  pname = "wl-screenrec";
  version = "0.2.0";

  inherit src;

  cargoLock = {
    lockFileContents = builtins.readFile ../Cargo.lock;
  };

  nativeBuildInputs = [
    pkg-config
    rustPlatform.bindgenHook
    installShellFiles
  ];

  buildInputs = [
    wayland
    libdrm
    ffmpeg
  ];

  doCheck = false;

  postInstall = lib.optionalString (stdenv.buildPlatform.canExecute stdenv.hostPlatform) ''
    installShellCompletion --cmd wl-screenrec \
      --bash <($out/bin/wl-screenrec --generate-completions bash) \
      --fish <($out/bin/wl-screenrec --generate-completions fish) \
      --zsh <($out/bin/wl-screenrec --generate-completions zsh)
  '';

  meta = {
    description = "High performance wlroots screen recording, featuring hardware encoding";
    homepage = "https://github.com/russelltg/wl-screenrec";
    license = lib.licenses.asl20;
    platforms = lib.platforms.linux;
    mainProgram = "wl-screenrec";
  };
}
