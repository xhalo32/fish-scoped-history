# Fish Scoped History

Key features:
- Implements the new `fish_history_api::HistoryProvider` API
- Provide directory-based scoping to command line history and autosuggestions
- Commands are stored in a SQLite database

## Planned features

- Configuration file
    - Global and local variants
    - Ability to specify which directories are "globally scoped" i.e. have scope set to the empty string.
    - Search ordering: are search results sorted based on scope or recency.
    - Detect repository as root: if a .git or .jj is found, then use that as the scope of commands run inside the directory
- Environment variables
    - `FISH_SCOPED_HISTORY_SCOPE=strict` only shows commands whose scope is equal to the CWD
    - `FISH_SCOPED_HISTORY_SCOPE=off` shows all commands, acts like standard history
    - `FISH_SCOPED_HISTORY_SCOPE=inverse` only shows commands whose scope starts with the CWD
- Incognito mode: completely disable storing history, but still allow reading it
