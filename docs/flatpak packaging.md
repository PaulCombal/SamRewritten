Flatpak packaging is currently not possible for SamRewritten for technical reasons.

I haven't tested IPC with Steam thoroughly, but it seems to be working fine.

The problem seems to be lying in the fact that Flatpak runs application as init processes.

When an App Server is started with Flatpak, even though the process exited, Steam won't recognize it as being closed. It
means we are marked as in-game all the time, even after SamRewritten is completely closed. (Probably because Steam waits
for PID 2 to exit or so).

Any help or advice would be greatly appreciated.