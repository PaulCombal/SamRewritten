SamRewritten
===

<p align=center>
    <img src="/assets/icon_256.png" alt="SamRewrittenLogo">
</p>

<p align=center>
    <img src="/assets/screenshot1.png" alt="SamRewritten screenshot">
    <em>GTK version preview</em>
</p>

<p align=center>
    <img src="/assets/screenshot2.png" alt="SamRewritten screenshot">
    <em>Adwaita version preview</em>
</p>

<p align="center">A Steam Achievement Manager for Windows and Linux.</p>
<p align="center">
    <a href="https://github.com/PaulCombal/SamRewritten/releases">DOWNLOAD</a>
</p>

<p align=center>
    <em>
        This project and its contributors are not affiliated with Valve Corporation or Microsoft.
        Steam and Windows are trademarks of their respective owners, Valve Corporation and Microsoft.
    </em>
</p>

## Thank you

SamRewritten is heavily inspired by other wonderful projects such
as [Steam Achievements Manager by Gibbed](https://github.com/gibbed/SteamAchievementManager)
or [Samira by jsnli](https://github.com/jsnli/Samira).
Thank you to all the contributors of these amazing projects, and
to [the legacy version of SamRewritten](https://github.com/PaulCombal/SamRewritten-legacy).

Most importantly, thank you to our awesome users and stargazers for giving us the motivation to keep building.

## What is SamRewritten?

SamRewritten is a tool that allows you to unlock and relock achievements on your Steam account. Additionally, it can edit stats for games and apps that expose them. While achievements carry no financial value, they are widely used for collection progress and "bragging rights"!

## Installation

Downloads are available on the [release tab](https://github.com/PaulCombal/SamRewritten/releases) for Windows (installer) and Linux (AppImage).

<details>
<summary>Click here for detailed Windows instructions</summary>

The recommended way to run SamRewritten on Windows is via the official installer.
You can download it from the Releases page. This is the only file you need; the other listed assets are not intended for general Windows use.
Once installed, you can find and launch SamRewritten through the Start menu.

If the installation fails or behaves unexpectedly, please report the issue by opening a GitHub issue with as much detail as possible, including your Windows version.

</details>

<details>
<summary>Click here for detailed Linux instructions</summary>

If your distribution does not provide a native package for SamRewritten, you can use our AppImages.
AppImages are self-contained executables designed to run on almost any Linux distribution.
Download the latest AppImage from the Releases page. To run it, ensure the file has execution permissions. You can usually do this by right-clicking the file, navigating to "Permissions," and checking the "Allow executing file as program" box.
You can then double-click the file to start the app.

If SamRewritten fails to launch, you can troubleshoot by running the AppImage from a terminal to see the logs.
To do this, open a terminal in your download folder and run the file directly (e.g., ./SamRewritten-gtk.AppImage).

If you see an error regarding "FUSE" or "libfuse," you may need to install the library:
```bash
sudo apt install libfuse2 # Example for Ubuntu/Debian
```

If issues persist, please open an issue including your distribution, version, and the console output from your terminal.

</details>

> [!NOTE]
> For Arch Linux and its derivatives, you can install SamRewritten with yay:
>
> `yay -S samrewritten-git`

## Features

* Lock and unlock specific achievements with a single click
* Bulk lock/unlock achievements for all or selected apps
* Schedule achievement unlocks over a custom period
* Edit statistics in real-time

## Limitations

⚠️ On Linux, this tool is **only** compatible with:
* Snap installations of Steam
* Ubuntu/Debian multiarch installations with apt
* Ubuntu/Debian installations with the .deb file from the official Steam website
* Distribution installations that use the Steam runtime (Gentoo, Arch, `~/.steam/root` exists)

If you would like to see your specific distribution supported, please open an issue.

> [!TIP]
> Flatpak support is a significant technical challenge. If you are familiar with Flatpak internals and would like to help, please reach out!

## CLI

SamRewritten also functions as a command-line tool. The CLI version does not require GTK.
For Windows users, the installer places the CLI version in the installation folder, though no shortcut is created for it.

The CLI allows you to:

* List apps, achievements, and stats
* Lock and unlock achievements
* Bulk lock and unlock achievements

## Environment variables

SamRewritten's behavior can be altered via environment variables:
* `SAM_APP_LIST_URL` which URL to download the app list from
* `SAM_STEAMCLIENT_PATH` (Linux only) override which `steamclient.so` file to load.
* `SAM_USER_GAME_STAT_SCHEMA_PREFIX` (Linux only) override the prefix of the app bin data files to load

## End User Agreement

This software is provided as a Proof-of-Concept. Users are solely responsible for any actions taken with this tool.
By using SamRewritten, you acknowledge that you alone are responsible for the management of your Steam account. None of the contributors shall be held liable for any repercussions resulting from the use of this software.

Using this tool in multiplayer games is highly discouraged.