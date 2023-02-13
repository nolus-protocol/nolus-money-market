# Release Policy

## General
1. Cargo package versions follow the [Semver](https://semver.org) semantic model.
2. A Cargo package is either library or contract package. The former are located under `packages` directory, the latter are under `contracts` directory.
3. An update of a package results recursively in updates of all packages up the dependency tree, e.g. the ones that dependent on the package directly or indirectly.
4. Library packages may depend on other library packages and never on contract packages.
5. Contract packages may depend on library packages and on stub-featured other contract packages. The latter means that the former contract calls, by sending query or execute messages, the latter contract.
6. There are dependencies to services or features provided by the layer 1. They cannot be expressed explicitly as Cargo dependencies.

## Contracts
7. Each Cargo contract package defines one and only one CosmWasm contract.
8. The format of the persistent data of a contract is tagged with a storage version. It is denoted with a monotonically increased unsigned integer beginning with zero.
9. A contract version is a pair of its storage and Cargo package versions. Let's denote with `V` contract versions, with `S` storage versions and with `P` package versions.
10. An update of the storage version must be accompanied by an update of the Cargo package version.
11. `V1 = <S1,P1>` *is-before* `V2 = <S2,P2>` only when `S1 < S2` or `S1 == S2 && V1 < V2`

## Releases
12. A new release is always associated with a Git Tag. The opposite is not mandatory although highly recommended.
13. Releases are **immutable**. Any modifications to the code of a released package must be released as a new version and included in the next release.
14. A new release encompasses the latest versions of each Cargo package either new ones or the same as they have appeared in the previous release. This uniquely defines the versions of contracts included in a release.
15. The only supported release updates are from the previous to the next release. An update to a newer release should be performed sequentially, one by one.

## Networks
16. Each Nolus Network runs a specific release. A direct corollary is that a network runs the contract versions as they appear in the release.
17. Ideally, an update to а new release should happen atomically to guarantee high availability of the services.

# Process
## TBD

# Useful commands
## When bumping Cargo packages
Use this to list the updated Cargo packages since the last or specified Git tag or reference

```bash
cargo workspaces changed -l [--since <ref>]
```

Use this to see the dependency tree of a Cargo package.

```
cargo tree -p <package>
```