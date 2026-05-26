SamRewritten
===

<p align=center>
    <img src="/assets/icon_256.png" alt="SamRewrittenLogo">
</p>

<p align=center>
    <img src="/docs/img/screenshot1.png" alt="SamRewritten screenshot">
    <em>GTK version preview</em>
</p>

<p align=center>
    <img src="/docs/img/screenshot2.png" alt="SamRewritten screenshot">
    <em>Adwaita version preview</em>
</p>

<p align="center">A Steam Achievement Manager for Windows and Linux.</p>
<p align="center">
    <a href="https://github.com/PaulCombal/SamRewritten/releases">DOWNLOAD</a>
</p>

<p align="center">
    <a href="#installation">Installation</a> ·
    <a href="#features">Features</a> ·
    <a href="#steam-compatibility">Steam compatibility</a> ·
    <a href="#cli">CLI</a> ·
    <a href="#translations">Translations</a>
</p>

<p align=center>
    <em>
        This project and its contributors are not affiliated with Valve Corporation or Microsoft.
        Steam and Windows are trademarks of their respective owners, Valve Corporation and Microsoft.
    </em>
</p>

## Acknowledgments

SamRewritten is heavily inspired by other wonderful projects such
as [Steam Achievements Manager by Gibbed](https://github.com/gibbed/SteamAchievementManager)
or [Samira by jsnli](https://github.com/jsnli/Samira).
Thank you to all the contributors of these amazing projects, and
to [the legacy version of SamRewritten](https://github.com/PaulCombal/SamRewritten-legacy).

Most importantly, thank you to our awesome users and stargazers for giving us the motivation to keep building.

## What is SamRewritten?

SamRewritten is a tool that allows you to unlock and relock achievements on your Steam account. Additionally, it can edit stats for games and apps that expose them. While achievements carry no financial value, they are widely used for collection progress and "bragging rights"!

## Installation

<details>
<summary><b>Arch Linux (AUR)</b></summary>

SamRewritten is available on the [AUR](https://aur.archlinux.org/packages/samrewritten-git). Install it with any AUR helper:

```bash
yay -S samrewritten-git
# or
paru -S samrewritten-git
```

</details>

<details>
<summary><b>Ubuntu Linux (Snap - Soon!)</b></summary>

**⚠️ Important:** The Snap version of SamRewritten cannot be used with the Flatpak version of Steam.

SamRewritten is published on the Snap Store and works on Ubuntu and any distribution with `snapd` installed:

```bash
sudo snap install samrewritten
```

[![Get it from the Snap Store](https://snapcraft.io/en/dark/install.svg)](https://snapcraft.io/samrewritten)

The snap ships three commands: `samrewritten` (GTK), `samrewritten.samrewritten-adw` (libadwaita), and `samrewritten.samrewritten-cli`. The CLI reuses the Steam folder granted by the GUI, so run a GUI command once first.

</details>

<details>
<summary><b>All Linux distributions (AppImage)</b></summary>

For any Linux distribution, you can use the AppImage. AppImages are self-contained executables that run on almost any Linux distribution.

1. Download the latest AppImage from the [Releases page](https://github.com/PaulCombal/SamRewritten/releases).
2. Make it executable. You can either right-click the file → "Permissions" → check "Allow executing file as program", or run:
   ```bash
   chmod +x SamRewritten-gtk.AppImage
   ```
3. Double-click the file to launch, or run it from a terminal:
   ```bash
   ./SamRewritten-gtk.AppImage
   ```

If SamRewritten fails to launch, run it from a terminal to see the logs. If you see an error mentioning "FUSE" or "libfuse", install the library:

```bash
sudo apt install libfuse2   # Ubuntu/Debian
```

If issues persist, please open an issue including your distribution, version, and the console output.

</details>

<details>
<summary><b>Windows</b></summary>

The recommended way to run SamRewritten on Windows is via the official installer, available on the [Releases page](https://github.com/PaulCombal/SamRewritten/releases). Once installed, you can find and launch SamRewritten through the Start menu.

A portable ZIP build is also provided on the Releases page if you prefer not to run an installer.

If the installation fails or behaves unexpectedly, please open a GitHub issue with as much detail as possible, including your Windows version.

</details>

## Features

* Lock and unlock specific achievements with a single click
* Bulk lock/unlock achievements for all or selected apps
* Schedule achievement unlocks over a custom period
* Edit statistics in real-time
* Per-app or bulk import and export of achievements and stats
* Idle apps: Appear in-game until you toggle it off
* A light and dark theme

## Translations

SamRewritten is translated by its community, and **we'd love your help** to
support more languages. You don't need to be a programmer — translating is just
filling in a text file, and you can submit it by opening an issue or a PR.

See [`po/README.md`](po/README.md) for a short, step-by-step guide.

## Steam compatibility

On Linux, SamRewritten supports all of these Steam installation types:

✅ Snap installations of Steam <br>
✅ Flatpak installations of Steam <br>
✅ Ubuntu/Debian multiarch installations with apt <br>
✅ Ubuntu/Debian installations with the .deb file from the official Steam website <br>
✅ Distribution installations that use the Steam runtime (Gentoo, Arch, `~/.steam/root` exists)

> [!NOTE]
> The Snap package supports every type above **except Flatpak Steam**. For a Flatpak Steam install, use the AppImage or [AUR](https://aur.archlinux.org/packages/samrewritten-git) package.

If you would like to see your specific distribution supported, please open an issue.

## CLI

SamRewritten also functions as a command-line tool. The CLI version does not require GTK.
For Windows users, the installer places the CLI version in the installation folder, though no shortcut is created for it.

The CLI allows you to:

* List apps, achievements, and stats
* Lock and unlock achievements
* Bulk lock and unlock achievements
* Import and export achievements and stats as JSON (compatible with the GUI format)
* Idle apps: Appear in-game until SIGINT (Ctrl+C)

When using a graphical version of SamRewritten, you can use `--auto-open=X` where `X` is an AppId, to open SamRewritten
directly on the corresponding app's details page.

## Environment variables

SamRewritten's behavior can be altered via environment variables:
* `SAM_STEAM_INSTALL_ROOT` (Linux only) override the detected Steam installation root path.
* `SAM_STEAMCLIENT_PATH` (Linux only) override which `steamclient.so` file to load.
* `SAM_GSCHEMA_DIR_FALLBACK` Fallback path for the `gschema.compiled` directory.
* `SAM_APP_LIST_URL` which URL to download the app list from

## End User Agreement

This software is provided as a Proof-of-Concept. Users are solely responsible for any actions taken with this tool.
By using SamRewritten, you acknowledge that you alone are responsible for the management of your Steam account. None of the contributors shall be held liable for any repercussions resulting from the use of this software.

Using this tool in multiplayer games is highly discouraged.