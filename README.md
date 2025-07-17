SamRewrittem
===

<p align=center>
    <img src="/assets/icon_256.png" alt="SamRewrittenLogo">
</p>

<p align=center>
    <img src="/assets/screenshot1.png" alt="SamRewritten screenshot">
</p>

<p align=center>
    <img src="/assets/screenshot2.png" alt="SamRewritten screenshot">
</p>

<p align="center">A Steam Achievements Manager for Windows and Linux.</p>
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
Thanks to all the contributors of these amazing repositories, and also
to [the legacy version of this very project](https://github.com/PaulCombal/SamRewritten-legacy).

And most of all, thank you to all our awesome users and stargazers, giving us motivation to keep building.

## What is SamRewritten?

SamRewritten is a tool that allows you to unlock, and lock again achievements on your Steam account.
Additionally, some apps and games expose stats which can also be edited using this tool. Achievements do not have any
financial value, however they are very desirable for bragging rights!

## Installation

Downloads are available in the [release tab](https://github.com/PaulCombal/SamRewritten/releases) for Windows (installer) and Linux (AppImage).

<details>
<summary>Click here to see detailed instructions for Windows</summary>

The supported way to run SamRewritten on Windows is by using the installer. 
You can download the installer at the Releases page.
This is the only thing you need to download; the other files are not meant to provide this program for Windows.
After running the installer and completing the installation, SamRewritten should appear and can be searched for via the start menu.

If the installation does not complete as intended, feel free to report it by opening an issue and providing as much details
as possible, including your version of Windows.
</details>

<details>
<summary>Click here to see detailed instructions for Linux</summary>

If your Linux distribution doesn't provide a way to install SamRewritten, you can use AppImages.
AppImages are self-contained executables designed to run independently of your Linux distribution.
AppImages for SamRewritten are available to download at the Releases page.
To run an AppImage, make sure you have the permission to execute it first. This can usually be confirmed by right-clicking 
the file, navigating to the permissions settings, and making sure the permission to run the file box is checked.
You should then be able to double-click the AppImage file to start SamRewritten.

If SamRewritten doesn't start, you can troubleshoot the issue by starting the AppImage via a terminal and examine the output.
To do so, open a terminal via your file manager in the same folder than your AppImage download and type the name of the file 
to start it (eg: `./SamRewritten-gtk.AppImage`).

If the message in the console mentions Fuse or libfuse, you might need to install it and try again:
```shell
sudo apt install libfuse2 # Example for Ubuntu/Debian
```

If the error persists, feel free to open an issue including your Linux distribution and its version, as well as the
console output that appeared after typing the name of the AppImage in your terminal. 
</details>

> [!NOTE]
> For Arch linux and derivatives, you can install SamRewritten with yay:
>
> `yay -S samrewritten-git`

<!--
Additionally, Snap users can install SamRewritten using the App store or with the following command:
```bash
snap install samrewritten
```
-->

## Features

* Lock and unlock select achievements with a single click
* Edit statistics instantly
* Schedule achievement unlocking over a set period of time

## Limitations

⚠️ On Linux, this tool is **only** compatible with:
* Snap installations of Steam
* Ubuntu/Debian multiarch installations with apt
* Distribution installations that use the Steam runtime (Gentoo, Arch, `~/.steam/root` exists)

If you wish to see your distribution supported, please open an issue.

> [!TIP]  
> Flatpak support poses a considerable challenge. If you or someone you know with knowledge of Flatpak internals can offer to help, please don't hold back and reach out!

## End user agreements

This software serves as a Proof-of-Concept. Users are responsible of their actions using this tool.
By using this tools, you agree that you are the only responsible to the management of your Steam account. None of the
contributors can be held responsible for the actions and their repercussions you have done using this tool.

Using this tool on multiplayer games is highly discouraged.
