## Global architecture schema

![Architectural software schema](samdoc.drawio.png)

* Client UI: GTK-based UI. Does not load any Steam-related binaries.
* Orchestrator: Loads the Steamworks SDK without an AppId. With limited capabilities. Is in charge of discovering the
  Steam installation, owned games, etc.
* App Servers: App processes. In charge of idling, retrieving and storing achievements and stats, etc.

Communications are made via pipes and work in a request-response fashion.
While crates like bincode could be used for performance gains, JSON was still chosen for its ease of use and human
readability. It wasn't found that this posed a significant bottleneck.

The reason why the orchestrator doesn't execute the game functions itself is because
Steam will still show you as being "in game" as long as the game process you started didn't finish,
and its zombie process waited.

## CLI mode

For the CLI mode, the above limitations do not apply. Every process represents a single app.
Once the process is killed, the app is killed with it, another app cannot be started in the same process.
An orchestrator is not needed.

## Code folders

* backend
    * Orchestrator and app servers
* gui_frontend
    * GTK/ADW Client UI (not needed for CLI mode)
* cli_frontend
  * Clap CLI Client UI (not needed for GUI mode)
* steam_client
    * Steamworks SDK bindings, used by the backend and orchestrator
* utils
    * Regular functions used by other modules, mostly for file path functions and IPC types.
    * Contains steam_locator.rs, which contains the logic for choosing paths for various reasons (loading steam, resources, ...)