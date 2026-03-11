# lapctl

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=flat&logo=rust&logoColor=white)
![Linux](https://img.shields.io/badge/Linux-FCC624?style=flat&logo=linux&logoColor=black)

**lapctl** is a fast and easy command line tool made in Rust. It helps you control your Linux laptop hardware from the terminal.

You can use it to change your graphics card mode, set battery charge limits, pick power profiles, and control the cooling fans. It talks directly to your system hardware, so it does not need any heavy background programs to run.

---

## What It Can Do

**Graphics Control**
Easily switch between Integrated, Nvidia, and Hybrid graphics modes to save battery or get more power.

**Battery Health**
Set a battery charge limit like 80% to make your battery last longer over the years.

**Power Saving**
Change power profiles and set CPU power limits to save energy when you need it.

**Cooling Control**
Control the cooling fans for laptops like Lenovo or ASUS to keep your computer quiet or cool.

**Hardware Status**
Check the status of your battery, graphics, and power all in one place.

---

## How to Use It

```bash
# Graphics
lapctl gpu integrated
lapctl gpu hybrid
lapctl gpu nvidia

# Battery limit
lapctl battery limit 80
lapctl battery status

# Power mode
lapctl power performance
lapctl power balanced
lapctl power battery-save

# Power limit in Watts
lapctl power limit-tdp 35

# Cooling fans
lapctl cooling performance
lapctl cooling balanced
lapctl cooling quiet

# Check everything
lapctl status
```

---

## File Layout

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
│   ├── lib.rs
│   │
│   ├── commands/
│   │   ├── mod.rs
│   │   ├── gpu.rs
│   │   ├── battery.rs
│   │   ├── power.rs
│   │   ├── cooling.rs
│   │   ├── install_rules.rs
│   │   └── status.rs
│   │
│   ├── hardware/
│   │   ├── mod.rs
│   │   └── gpu.rs
│   │
│   └── utils/
│       └── system.rs
│
└── tests/
    ├── cli.rs
    └── gpu.rs
```

---

## Project Goals Finished

[x] CLI setup and routing
[x] Graphics switching
[x] Battery limits tools
[x] System status checking
[x] Tool configuration saving
[x] Cooling fan setup
[x] CPU power limit setup

---

## Contributions

We welcome your ideas and help!
To contribute, please fork the repository, create a new feature branch, and submit a Pull Request. Please ensure all tests pass by running `cargo test` before submitting your changes.

---

## Special Credit

Special thanks to [EnvyControl](https://github.com/bayasdev/envycontrol) by bayasdev. `lapctl` uses many ideas from EnvyControl to handle graphics setup on Linux.

---

## License

This project uses the **MIT License**.

You can use, change, and share this software freely under the rules of the license.
Check the [LICENSE](LICENSE) file in this folder for details.

---

⭐ If you like this tool, please consider starring the repository!
