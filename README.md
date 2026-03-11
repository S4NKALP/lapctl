# lapctl

**lapctl** is an upcoming command-line utility written in Rust for managing laptop hardware features on Linux.

It aims to provide a simple and unified interface for controlling hybrid GPU modes, battery charging limits, and other laptop power features directly from the terminal.

---

## Planned Features

- Switch GPU modes on hybrid graphics laptops
  - Integrated GPU
  - Hybrid mode
  - NVIDIA GPU

- Set and manage battery charging limits
- View laptop hardware and power status
- Lightweight and fast CLI interface
- Designed for Linux systems with hybrid GPUs (Optimus)

---

## Project Status

**Work in progress**

This project is currently in early development.
Features, commands may change as development progresses.

---

## Goals

- Provide a **simple CLI interface** for laptop hardware management
- Support **hybrid GPU laptops (Intel + NVIDIA / AMD + NVIDIA)**
- Offer **battery health features like charge limit control**
- Be **lightweight, fast, and easy to use**

---

## Example CLI (Planned)

```bash
lapctl gpu integrated
lapctl gpu hybrid
lapctl gpu nvidia

lapctl battery limit 80
lapctl battery status

lapctl status
```

---

## Built With

- Rust
- Modern Linux system interfaces

---

## Project Structure

```
lapctl
│
├── Cargo.toml
├── README.md
├── LICENSE
├── .gitignore
│
├── src/
│   ├── main.rs
│   ├── cli.rs
│   │
│   ├── commands/
│   │   ├── mod.rs
│   │   ├── gpu.rs
│   │   ├── battery.rs
│   │   ├── power.rs
│   │   └── status.rs
│   │
│   ├── hardware/
│   │   ├── mod.rs
│   │   ├── gpu.rs
│   │   └── battery.rs
│   │
│   └── utils/
│       └── system.rs

```

## Roadmap

- [ ] CLI structure
- [ ] GPU mode switching
- [ ] Battery charge limit management
- [ ] Hardware status command
- [ ] Configuration support

---

## Contributing

Contributions, ideas, and feature suggestions are welcome.
More contribution guidelines will be added once the project reaches its first stable milestone.

---

## License

This project is licensed under the **GNU Affero General Public License v3.0 (AGPL-3.0)**.

You are free to use, modify, and distribute this software under the terms of the license.
For full details, see the [LICENSE](LICENSE) file in this repository.

---

⭐ If you find this project interesting, consider starring the repository to follow its progress.
