<div align="center">

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=flat&logo=rust&logoColor=white)
![Linux](https://img.shields.io/badge/Linux-FCC624?style=flat&logo=linux&logoColor=black)

**Take full control of your Linux laptop hardware with a fast, zero dependency CLI tool built in Rust.**

</div>

---

### Why lapctl?

Built with performance and simplicity in mind, it talks directly to your system's hardware interfaces (`sysfs`, `acpi`, `udev`). A high-performance background daemon (**lapctld**) handles all privileged operations via a secure D-Bus interface, allowing you to control your laptop completely **without sudo**.

---

### Key Features

- **Graphics Switching**: Effortlessly toggle between Integrated, NVIDIA, and Hybrid modes. Optimize for battery life on the go or raw performance at your desk.
- **Battery Health**: Modern batteries hate being at 100% all the time. Set custom charge limits (like 80%) to significantly extend your battery's lifespan.
- **Power Tuning**: Switch through performance profiles or set hard CPU power (TDP) limits in Watts to keep things cool or let them loose.
- **Intelligent Cooling**: Force your fans into Performance, Balanced, or Quiet modes (supporting ASUS and Lenovo laptops).
- **Display Refresh Rate**: Easily query available refresh rates and change your active display's Hz on-the-fly (100% native Rust Wayland implementation using `zwlr_output_manager_v1` for wlroots compositors like Sway and Hyprland).
- **Touchpad Toggle**: Quickly enable or disable your touchpad from the terminal when using an external mouse.
- **Sleep Inhibitor**: Running a long compile or a critical download? Use the inhibitor to stop your laptop from falling asleep mid task.
- **Instant Status**: Get a bird's eye view of your hardware state, battery health, and current limits with one simple command.

---

### Installation

**lapctl** is built with Rust. Ensure you have the [Rust toolchain](https://rustup.rs/) installed.

#### Option 1: Arch Linux (AUR)

If you are on Arch Linux, you can install **lapctl-git** from the AUR. This is the recommended method for Arch users as it automatically handles dependencies and udev rules.

```bash
yay -S lapctl-git
```

#### Option 2: Using `just` (Recommended for other distros)

If you have [just](https://github.com/casey/just) installed:

```bash
just install
sudo lapctl install-rules
```

*The `install-rules` command automatically sets up the D-Bus policy and starts the `lapctld` background service.*

#### Option 3: Using `cargo`

```bash
cargo install --path .
sudo lapctl install-rules
```

#### Requirements

- **systemd**: For sleep inhibitor (`systemd-inhibit`)
- **GPU Switching (Optional)**: `xrandr` and `nvidia-settings` are strictly required **ONLY** when using the `lapctl gpu` command on X11 (to route proprietary NVIDIA Optimus drivers).
- **Wayland Display**: Built entirely natively using `wayland-client` and `wayland-protocols-wlr` (no `wlr-randr` required!)

#### Limitations

- **GNOME / KDE Plasma (Wayland)**: The display refresh rate feature relies heavily on the `zwlr_output_manager_v1` protocol. This protocol is exclusive to wlroots-based compositors (like Sway and Hyprland). GNOME and KDE use their own disparate internal display protocols, meaning this feature will **not work** out-of-the-box on those Desktop Environments.

---

### Quick Start Guide

```bash
# Install D-Bus policy & start daemon (Required for rootless control)
sudo lapctl install-rules

# From now on, NO sudo is required!

# Manage your GPU
lapctl gpu integrated  # Max battery
lapctl gpu hybrid      # Best of both worlds
lapctl gpu nvidia      # High performance
lapctl gpu run steam   # Run 'steam' on dGPU directly while in Hybrid mode

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

# Manage your touchpad
lapctl touchpad disable
lapctl touchpad enable

# Manage your display refresh rate
lapctl display rates
lapctl display set-rate 144

# Keep it awake (requires lapctld)
lapctl inhibit --daemon  # Stay awake persistently (via daemon)
lapctl inhibit --stop    # Allow sleep again
lapctl inhibit --why "Keeping awake" -- sleep 60 # One-shot (local)

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
│   ├── daemon/         # lapctld (D-Bus interface & logic)
│   ├── commands/       # CLI feature logic & proxies
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
