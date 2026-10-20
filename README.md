# EonOS external apps

This folder is for applications developed separately from EonOS's bundled
system programs. Each app has its own Cargo project and can be built into a
`.eonapp` package with the repository's app builder.

Build the example:

```sh
./app.sh build eon-apps/hello-world
```

This writes `build/apps/hello-world.eonapp`. A copy is kept here as
`eon-apps/hello-world.eonapp` for easy access. Copy the package to EonOS's
mounted data disk, then install it from Konsul or the shell:

```text
install /disk/hello-world.eonapp
```

The example is an `ERA` app and uses the safe window API from `eonos-app`.
