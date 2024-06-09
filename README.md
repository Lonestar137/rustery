
# Rustery

A simple CLI tool for automating the naming, building, and tagging of
docker/podman images.

## Installation
Download the rust toolchain from [https://rustup.rs/](https://rustup.rs/) 
and install the code:

```bash
cargo install --git https://github.com/Lonestar137/rustery
```

Don't forget to add `$HOME/.cargo/bin` to your systems path.

## CLI

```bash
Automatically orchestrates container builds

Usage: rustery [OPTIONS]

Options:
  -c, --client <CLIENT>        CLI container client to use [default: podman]
  -b, --basepath <BASEPATH>    Directory to scan for containerfiles in [default: .]
  -e, --extension <EXTENSION>  File extension of containerfiles [default: docker]
  -r, --registry <REGISTRY>    Remote registry to push built images to
  -d, --dryrun                 Dryrun
  -h, --help                   Print help
  -V, --version                Print version
  ```

## Usage example

Create a folder with the following structure:

```
python/base__38.docker
myapp/app__1.docker
```

The contents of `python/base_38.docker` are:
```dockerfile
FROM python:latest

# Any other custom images steps here.
```

and `myapp/app__1.docker`:
```dockerfile
FROM localhost/python/base:38
# Any other custom images steps here.
```

From the root of this structure, we can do a dryrun with the command:
```bash
rustery --dryrun
```

The output should look like so:
```bash
podman pull python:latest
podman build --file ./python/base__38.docker --tag localhost/python/base:38 --format docker .
podman build --file ./myapp/app__1.docker --tag localhost/myapp/app:1 --format docker .
```

Basically, we have created a relationship between the two files.  The file 
**python/base** is a requirement of **myapp/app**, so we ensure that it is 
built first.

As you can imagine, this makes it easier to scale and build your own registry 
as a single git repository.  Images can be pushed to a remote registry as they
are built by passing the `--registry` argument and specifying an endpoint.

