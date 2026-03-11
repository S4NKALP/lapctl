# lapctl

**lapctl** is a powerful, dependency-free command-line utility written in Rust. It is designed to provide seamless hardware management for Linux laptops. 

By interfacing directly with the Linux kernel and ACPI endpoints, `lapctl` provides a unified interface for controlling hybrid GPU modes, battery charging thresholds, and thermal/power profiles—all from the terminal.

---

## Key Features

- **Hybrid GPU Management:** Switch effortlessly between Integrated, Nvidia, and Hybrid graphics modes (Optimus).
- **Battery Health:** Enforce battery charging thresholds (e.g., 80%) to maximize chemical lifespan.
- **Power Profiling:** Dynamically adjust ACPI Platform Profiles, CPU scaling governors, and TDP (Wattage) limits.
- **Thermal Controls:** Direct hardware hooks for proprietary cooling solutions (e.g., Lenovo Ideapad, ASUS).
- **Hardware Polling:** Comprehensive hardware status checking without heavy daemon dependencies.

---

## Example CLI

```bash
# GPU Modes
lapctl gpu integrated
lapctl gpu hybrid
lapctl gpu nvidia

# Battery Controls
lapctl battery limit 80
lapctl battery status

# Power Profiles
lapctl power performance
lapctl power balanced
lapctl power battery-save

# Raw Wattage Capping
lapctl power limit-tdp 35

# Cooling/Thermal Overrides (Lenovo/ASUS)
lapctl cooling performance
lapctl cooling balanced
lapctl cooling quiet

# Check All Hardware
lapctl status

# --- lapctl status ---
# GPU Mode: hybrid
# BAT1:
#   Capacity: 99%
#   Status: Full
#   Charge Limit: 100%
# Power Profile: balanced
# Cooling Profile: Balanced (Ideapad)
# CPU TDP Limit: Hardware Managed
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
│   │   ├── cooling.rs
│   │   └── status.rs
│   │
│   ├── hardware/
│   │   ├── mod.rs
│   │   └── gpu.rs
│   │
│   └── utils/
│       └── system.rs

```

## Core Capabilities

- [x] CLI structure and routing architecture
- [x] GPU mode switching and configuration orchestration
- [x] Battery charge limit management (sysfs & Ideapad conservation mode)
- [x] Comprehensive hardware status polling
- [x] Configuration support and state caching
- [x] Thermal/Cooling profiles (Lenovo VPC / ASUS WMI)
- [x] CPU TDP (Wattage) Control (Intel RAPL / AMD hwmon)

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
