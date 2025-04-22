# usbvfiod

**usbvfiod** is a Rust-based tool designed to enable USB device
passthrough to [Cloud
Hypervisor](https://github.com/cloud-hypervisor/cloud-hypervisor)
virtual machines using the [vfio-user
protocol](https://github.com/tmakatos/qemu/blob/master/docs/devel/vfio-user.rst). Other
VMMs might also work, but but are currently not the main target.

This project is still under active development and **not usable
yet**. We are planning to work on this project in the following order:

1. **Validating our Assumptions** (🚧 **Ongoing** 🚧)
   - We are looking for suitable libraries to use and finalize our design.
2. **Towards USB Storage Passthrough**
   - We build up a virtual XHCI controller and the necessary plumbing
     to pass-through USB devices from the host.
   - Our initial test target will be USB storage devices.
3. **Broaden Device Support**
   - We broaden the set of USB devices we support and actively test.

If you want to use this code, please check back later or [get in
touch](https://cyberus-technology.de/en/contact), if you need
professional support.

## Documentation

Find the overview of documentation [here](./docs/overview.md).

## Development

The following section is meant for developers.

### Format Checks

`.toml` files in the repository are formatted using
[taplo](https://taplo.tamasfe.dev/). To re-format `.toml` files, you
can use:

```console
$ taplo format file.toml
```
