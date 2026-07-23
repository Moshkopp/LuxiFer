# MkStudio

> [!WARNING]
> **MkStudio is at an early and immature stage of development.**
> It is not yet approved for production use with connected machinery.
> Unexpected machine movement or laser activation may occur.
>
> **Use of this software and any connected hardware is entirely at your own
> responsibility and risk.** MkStudio does not replace suitable machine
> safeguards, an emergency stop, enclosure, extraction, eye protection,
> supervision, or the safety instructions provided by the machine and laser
> manufacturers.

MkStudio is a free, native application for laser design, job preparation, and
machine control. Its optional local Hub coordinates project revisions, assets,
settings backups, and shared access between workstations. The project is under
active development and emphasizes a clear separation between the user
interface, application logic, device-independent core, and machine-specific
drivers.

## Development status

MkStudio is currently experimental. There is no general release for operating
a connected machine or laser source yet.

| Area | Status |
|---|---|
| Native editor | Under active development |
| Ruida | Separate driver and transport, under active development |
| grblHAL | Serial, console, buffered streaming, live status, and stop under development |
| Mini/classic GRBL | Shared GRBL family with a compatible stop strategy |
| FluidNC | Planned as another GRBL-family strategy |
| Ethernet for the GRBL family | Planned |
| MkStudio Hub | Optional local coordination and synchronization service, under active development |

Hardware testing is introduced gradually and initially performed without a
connected laser source. The current technical status and documented hardware
tests are tracked in the [grblHAL roadmap](docs/roadmap/grblhal.md).

## Safety

Before any hardware test, at least the following precautions should be taken:

- Disconnect the laser source or reliably limit it to `S0`/zero power unless
  laser output is explicitly required by the test.
- Keep a physical emergency stop or suitable power disconnect within reach.
- Never operate the machine unattended.
- Keep the work area clear and allow for unexpected axis movement.
- Use a suitable enclosure, extraction, eye protection, and fire precautions.
- Independently verify configuration, coordinate systems, limit switches, and
  power limits before starting a job.

A successful test on one controller does not constitute approval for other
firmware versions, electronics, machines, or laser sources.

## Build and run

An up-to-date stable [Rust toolchain](https://www.rust-lang.org/tools/install)
is required. Development and hardware testing currently take place on Linux.
Other platforms are not yet documented as supported targets.

```bash
cargo build --workspace
cargo test --workspace
cargo run --release -p studio
```

To create a release build of the application:

```bash
cargo build --release -p studio
```

The optional local Hub can be started separately:

```bash
cargo run --release -p hub
```

Additional native packages for windowing, graphics acceleration, and serial
devices may be required depending on the operating system.

## Architecture

```text
studio/native       User interface and presentation
        ↓
studio/application  Use cases and device lifecycle
        ↓
studio/core         Device-independent models and intents
        ↓
studio/drivers      Ruida driver and GRBL driver family

hub                 Optional local coordination and synchronization service
```

The UI does not generate GRBL, Ruida, or serial protocol commands. Ruida
remains a fully independent driver. grblHAL, Mini/classic GRBL, and eventually
FluidNC share only the common parts of their protocol family and use separate
strategies for actual differences.

The Hub distributes project revisions and assets, stores workstation backups,
and coordinates shared Ethernet-device leases. It is never required for local
editing or saving, and it does not send jobs or machine commands.

Long-term architectural decisions are documented in [docs/adr](docs/adr).

## Contributing

Bug reports, reproducible hardware observations, documentation, and
contributions are welcome. For changes that affect machinery, please include
the controller, firmware version, connection type, and safe test conditions.
Do not assume tests with real laser output or suggest them without a clear
warning.

## License

MkStudio is released under the
[GNU General Public License Version 3](LICENSE), version 3 only
(`GPL-3.0-only`).

Dependencies and bundled third-party components retain their respective
licenses.
