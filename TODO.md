### Medium priority

- Instead of having small bars under the achievements to represent gloable achievement percentage, can we make it the
  background? would it look any better?
- Support for multiple simultaneous instances ('launch in new window' button)
- In utils, if the registry key isn't found still return a string, don't panic.

### Low priority

- Show achievement progression when available
- Improve error handling (handle .expects, .unwraps, etc)

### Nices to have

- In utils, instead of calculating Steam install path every time, do it only once

### Help needed

- Support for Flatpak installs of Steam
- Fix snapcraft packaging
- Find a solution to this problem: https://github.com/PaulCombal/achievement-poc

### Will not implement

- Support for installs of Steam via package manager (.debs, ...). They require 32bits system packages and should be
  considered legacy.