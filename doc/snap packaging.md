Snap packaging is currently not possible for SamRewritten for the following reasons:

* Packaging with classic confinement works, but is not allowed on the Snapcraft store.
* Packaging in strict confinement does not work, because of the reasons discussed
  here: https://forum.snapcraft.io/t/samrewritten/47964/10 .

As suggested in the forum thread, I tried copying the steamclient.so file in STEAM_USER_COMMON, and loading it from
there, but in this scenario, as soon as a child process (app server) loads the .so file, all the other processes stop
having access to the interfaces.

So while IPC is still doable, it is very unstable and prone to crashes.

I tried implementing symbol interposition to fake dladdr returning the original location from which the .so file is
supposed to be loaded, without any difference.

I only imagine two ways things can turn around:

1. Reverse-engineer steamclient.so enough to understand why this happens and work around it
2. Snap allows program to file_mmap from a personal-files location

Option 2 would be preferred, but overall both choices are out of my expertise or control. Any help would of course be
welcome.

For the time being, references to Snap will not be removed, in the hopes that Snap will mature over the years and enable
SamRewritten to be packaged.

The snapcraft.yaml file in this folder is kept for reference and can be used to pack SamRewritten using classic
confinement.
