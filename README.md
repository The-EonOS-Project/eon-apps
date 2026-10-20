# EonOS external apps

This folder is for applications developed separately from EonOS's bundled
system programs. Each app has its own Cargo project and can be built into a
`.eonapp` package with the repository's app builder.

Build the example from this repository's root:

```sh
./app.sh build hello-world
```

This writes `build/apps/hello-world.eonapp` and copies the package to
`hello-world.eonapp` for easy access. By default, this repo expects to be
checked out next to EonOS and uses its `userland/app-sdk` and target files. If
EonOS is elsewhere, set `EONOS_ROOT=/path/to/eonos`. Copy the package to EonOS's
mounted data disk, then install it from Konsul or the shell:

```text
install /disk/hello-world.eonapp
```

The example is an `ERA` app and uses the safe window API from `eonos-app`.
