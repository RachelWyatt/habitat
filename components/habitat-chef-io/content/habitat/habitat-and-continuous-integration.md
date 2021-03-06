+++
title = "Chef Habitat and Continuous Integration"
description = "Chef Habitat and Continuous Integration"

[menu]
  [menu.habitat]
    title = "Chef Habitat and Continuous Integration"
    identifier = "habitat/builder/habitat-and-continuous-integration"
    parent = "habitat/builder"
    weight = 20

+++

**Examples: [Jenkins](https://jenkins.io/), [TravisCI](https://travis-ci.org/), and [Drone](https://drone.io/)**

Continuous integration allows you to build, test, and deploy your code by using CLI tools and plugins. Chef Habitat includes the [Chef Habitat Studio](https://www.habitat.sh/docs/developing-packages/#plan-builds) which allows you to do interactive builds on your developer workstation, or non-interactive builds with your continuous integration server. Your continuous integration server can also call the Chef Habitat CLI to promote your Chef Habitat packages to different channels, enabling your applications to update themselves. Chef Habitat is not a continuous integration server and can make builds and promotion processes done by your continuous integration server easier.

The [Chef Habitat Studio](https://www.habitat.sh/docs/developing-packages/#plan-builds) provides a clean room build environment for your application build. In effect, builds that occur on a developer's workstation, or on a continuous integration server, will build in the same manner. Developers no longer need to worry about entire classes of "it works on my box" problems. Build engineers no longer need to create unique and difficult to maintain worker nodes for continuous integration servers. Instead, the Chef Habitat plan.sh file contains all the information needed to build the entire application, from dependency management, runtime environment binaries, packaging, and application lifecycle hooks. When using the [Chef Habitat Studio](https://www.habitat.sh/docs/developing-packages/#plan-builds), your continuous integration server can focus more on what it is good at doing, instead of worrying about managing custom plugins and their potential conflicts.

Your continuous integration server can promote a Chef Habitat package (a .hart file) to a channel by calling the [Chef Habitat CLI](https://www.habitat.sh/docs/install-habitat/#install-habitat). This promotion method allows you to deploy a new version of your application in a pull-based manner by using the Chef Habitat Supervisor. Because this promotion process can be invoked non-interactively through the [Chef Habitat CLI](https://www.habitat.sh/docs/install-habitat/#install-habitat), you can manage your deployments using your existing tooling. If you choose, you can also do this promotion process manually. More complex application environments can also invoke the promotion process using a scheduling tool or provisioning tool to help manage infrastructure resources in addition to promoting Chef Habitat packages.

