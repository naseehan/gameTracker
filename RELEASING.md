# Releasing GameTracker

`npm run tauri build` produces an NSIS installer in
`src-tauri/target/release/bundle/nsis/`. Before giving it to other people, set up the two
things below. Neither can be done in the code alone: they need keys that only you hold.

## 1. Code signing (removes the "unknown publisher" warning)

Unsigned installers trigger a Windows SmartScreen warning that most people won't click
through. You need a code-signing certificate:

- **Azure Trusted Signing** is the cheapest option for individuals (a monthly fee, no
  hardware token). Tauri's guide:
  https://tauri.app/distribute/sign/windows/#azure-trusted-signing
- An **OV or EV certificate** from a certificate authority also works. EV certificates get
  SmartScreen reputation immediately; OV certificates build it up over time.

Once you have one, add it to `bundle.windows` in `src-tauri/tauri.conf.json` as described in
the guide (`certificateThumbprint`, or a `signCommand` for Azure), and every build is signed.

## 2. Automatic updates

The updater is built in but stays off while `plugins.updater` in `tauri.conf.json` has an
empty `pubkey` and no `endpoints`. To turn it on:

1. Create an update signing key. This is separate from the code-signing certificate, and
   **if you lose it you can't ship updates to existing installs**, so back it up:

   ```bash
   npx tauri signer generate -w ~/.tauri/gametracker.key
   ```

2. Put the printed public key in `plugins.updater.pubkey`, and add an endpoint that serves
   the update manifest. With GitHub Releases that is:

   ```json
   "updater": {
     "pubkey": "<contents of gametracker.key.pub>",
     "endpoints": ["https://github.com/<owner>/<repo>/releases/latest/download/latest.json"]
   }
   ```

3. Set `"createUpdaterArtifacts": true` under `bundle`, then build with the private key
   available:

   ```bash
   set TAURI_SIGNING_PRIVATE_KEY_PATH=%USERPROFILE%\.tauri\gametracker.key
   set TAURI_SIGNING_PRIVATE_KEY_PASSWORD=<password you chose>
   npm run tauri build
   ```

4. Upload the installer, its `.sig` file and a `latest.json` manifest to the release.
   [tauri-action](https://github.com/tauri-apps/tauri-action) does all of this from GitHub
   Actions, including generating `latest.json`.

The app checks for updates at startup and shows "Install" when a new version is out; it can
also be checked from Settings → Help & About.

## 3. Before each release

- Bump `version` in `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml` and `package.json`.
- Run `cargo test` in `src-tauri` and `npx tsc --noEmit`.
- Install the build on a clean PC and play a game for a few minutes to check it's tracked.
