<div align="center">

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=flat&logo=rust&logoColor=white)
![Linux](https://img.shields.io/badge/Linux-FCC624?style=flat&logo=linux&logoColor=black)

**Take full control of your Linux laptop hardware with a fast, zero dependency CLI tool built in Rust.**

</div>

---

### Why lapctl?

Modern Linux laptops often have great hardware that goes underutilized or requires heavy, bloated background services to manage. **lapctl** changes that.

Built with performance and simplicity in mind, it talks directly to your system's hardware interfaces (`sysfs`, `acpi`, `udev`). No background daemons (unless you want them), no heavy RAM usage just pure control from your terminal.

---

### Key Features

- **Graphics Switching**: Effortlessly toggle between Integrated, NVIDIA, and Hybrid modes. Optimize for battery life on the go or raw performance at your desk.
- **Battery Health**: Modern batteries hate being at 100% all the time. Set custom charge limits (like 80%) to significantly extend your battery's lifespan.
- **Power Tuning**: Switch through performance profiles or set hard CPU power (TDP) limits in Watts to keep things cool or let them loose.
- **Intelligent Cooling**: Force your fans into Performance, Balanced, or Quiet modes (supporting ASUS and Lenovo laptops).
- **Sleep Inhibitor**: Running a long compile or a critical download? Use the inhibitor to stop your laptop from falling asleep mid task.
- **Instant Status**: Get a bird's eye view of your hardware state, battery health, and current limits with one simple command.

---

### Installation

**lapctl** is built with Rust. Ensure you have the [Rust toolchain](https://rustup.rs/) installed.

#### Option 1: Using `just` (Recommended)
If you have [just](https://github.com/casey/just) installed:
```bash
just install
```

#### Option 2: Using `cargo`
```bash
cargo install --path .
```

#### Requirements
- **systemd**: For sleep inhibitor (`systemd-inhibit`)
- **X11/NVIDIA Tools**: `xrandr`, `nvidia-settings` for GPU management

---

### Quick Start Guide

```bash
# Manage your GPU
lapctl gpu integrated  # Max battery
lapctl gpu hybrid      # Best of both worlds
lapctl gpu nvidia      # High performance

# Prolong battery life
lapctl battery limit 80
lapctl battery status

# Tune your power
lapctl power performance
lapctl power battery-save
lapctl power limit-tdp 35  # Stay under 35W

# Adjust your fans
lapctl cooling quiet
lapctl cooling performance

# Keep it awake
lapctl inhibit --daemon  # Run in background
lapctl inhibit -- why "Critical update" ./long-task.sh

# Check everything
lapctl status
```

---

### Under the Hood

The project is structured for speed and modularity:

```
lapctl
│
├── src/
│   ├── main.rs         # The entry point
│   ├── cli.rs          # Command definition & parsing
│   ├── commands/       # Feature logic (GPU, Battery, etc.)
│   ├── hardware/       # Hardware specific drivers (NVIDIA, etc.)
│   └── utils/          # System helpers
│
└── tests/              # Robust integration & unit tests
```

---

### Development & Connection

We love seeing how you use **lapctl**!

- **Contribute**: Found a bug or have a feature idea? [Open an issue](https://github.com/S4NKALP/lapctl/issues) or submit a Pull Request. We're always looking for help supporting more laptop brands!
- **Testing**: Please ensure all tests pass by running `cargo test` before submitting changes.
- **Shoutout**: Huge thanks to [EnvyControl](https://github.com/bayasdev/envycontrol) for the inspiration on graphics management.

---

### License

This project is licensed under the **MIT License**. You are free to use, modify, and distribute it as you see fit. See [LICENSE](LICENSE) for the full text.

---

<div align="center">
  <b>If you find lapctl useful, please consider starring the repository!</b>
</div>
