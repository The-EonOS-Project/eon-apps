# EonOS external apps

This folder is for applications developed separately from EonOS's bundled
system programs. Each app has its own Cargo project and can be built into a
`.eonapp` package with the repository's app builder. Current examples and
utilities are:

- `hello-world`: minimal Era window example.
- `crash-analyzer`: interprets `/disk/crashlog.txt` in an Era window.
- `pomodoro`: focus timer with a five-minute break phase.
- `minesweeper`: mouse-playable Era game.
- `sha256`: shell command that hashes a readable file up to the SDK's 1 MiB limit.
- `text-editor`: Era text editor with Open, Save, Save As, and unsaved-change prompts.

Build a package from this repository's root:

```sh
./app.sh build hello-world
```

This writes `build/apps/hello-world.eonapp` and copies the package beside its
project directory. Build each app with the same command:

```sh
./app.sh build crash-analyzer
./app.sh build pomodoro
./app.sh build sha256
./app.sh build minesweeper
./app.sh build text-editor
```

By default, this repo expects to be checked out next to EonOS and uses its
`userland/app-sdk` and target files. If EonOS is elsewhere, set
`EONOS_ROOT=/path/to/eonos`. A normal build writes the `.eonapp` beside its
app source and under `eon-apps/build/apps`. To include a package in an EonOS
image, direct the app builder to EonOS's build directory before running its
build script:

```sh
EONOS_BUILD_DIR="$(cd .. && pwd)/build" ./app.sh build text-editor
```

The image build copies those packages under `/packages`; install them from
Konsul or the shell (with `/disk` mounted):

```text
install /packages/crash-analyzer.eonapp
install /packages/pomodoro.eonapp
install /packages/sha256.eonapp
install /packages/minesweeper.eonapp
install /packages/text-editor.eonapp
```

Crash Report Analyzer reads `/disk/crashlog.txt`. Pomodoro Timer provides 15,
25, and 45 minute focus sessions with five-minute breaks. Minesweeper uses the
Era window and mouse APIs. The SHA-256 Checker is a `SHELL` app; launch it from
Konsul with `sha256 /disk/path/to/file`. It currently hashes readable files up
to the SDK's 1 MiB `read_file` limit. The interactive apps use the safe window
API from `eonos-app`.

Text Editor reads and writes UTF-8 text files under `/disk`, supports keyboard
editing and cursor movement, and prompts before discarding unsaved changes.
