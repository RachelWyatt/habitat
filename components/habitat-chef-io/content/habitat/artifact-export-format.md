+++
title = "Chef Habitat Artifact Export Formats"
description = "Chef Habitat Artifact Export Formats"

[menu]
  [menu.habitat]
    title = "Chef Habitat Artifact Export Formats"
    identifier = "habitat/packages/artifact-export-format Export Artifacts"
    parent = "habitat/packages"
    weight = 20

+++

Chef Habitat `.hart` files can be exported in a number of different formats depending on what you need and where you need it. This is powerful because you can use the same immutable Chef Habitat artifact by exporting it into a format that you need for a specific job. For example, when you can use one format for iterating locally in a Docker container, another to deploy that Chef Habitat artifact to an environment running Kubernetes, and a third to deploy it to a data center that's running virtual machines, but the Chef Habitat artifact is identical in each location+++it's simply exported to the correct format for the job you are trying to do.

You can read more about how to export Chef Habitat artifacts, and what exporters are currently available, [here](/docs/developing-packages/#pkg-exports).
