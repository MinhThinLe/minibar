# Minibar (name WIP)

A status bar for wayland that's meant to do less (with less, duh)

# Features

Currently, there's support for the following modules
- Battery
- Clock
- Cpu
- Memory
- Temperature
- Workspaces (Only supports Niri atm)
- Audio (Only supports pipewire atm)
- Group
- Bluetooth
- Backlight

Support for the following modules is planned
- Idle inhibitor
- Systray

# Configuration

Since the project is still in its infancy stages, the configuration format hasn't been finalized yet, though there is an [example config](./examples/configs/test-config.toml)

# Motivation

I wanted a status bar that has all the following features
- Is just a bar, not an entire desktop shell
- Good multi-monitor support (doesn't crash on monitor hotplug)
- Use as little system resource as possible (so I can LARP)

I have checked out the following projects

### [Waybar](https://github.com/Alexays/Waybar)
Pros:
- Is nothing more than a bar
- OK-ish resource consumption (roughly 100MB of RAM)

Cons:
- It sometimes crashes on monitor hot-plug. Not often enough to be easily debug-able, just enough to be annoying

### [**Noctalia**](https://github.com/noctalia-dev/noctalia)
Pros:
- Hot-plugging monitors never crashes the shell
- Just as light as the services it's meant to replace (Waybar + Swayidle + Hyprlock + SwayOSD + Mako + Rofi, etc)

Cons:
- Does too much

### [Ironbar](https://github.com/JakeStanger/ironbar)
Pros:
- Good customizability

Cons:
- Just as heavy as Waybar since it's written with GTK, might as well use Waybar at that point

### [Dwl's bar](https://codeberg.org/dwl/dwl-patches/src/branch/main/patches/bar)
Pros:
- Is just a bar, nothing more, nothing less
- *Tiny* footprint (~1 or 2 MB of RAM, can't be sure since it's a dwl patch rather than a separate program)

Cons:
- Is tied to dwl

### [Swaybar](https://github.com/swaywm/sway/tree/master/swaybar)
Pros:
- Small footprint (17MB of RAM)
- Also have other things I need (like a systray)

Cons:
- Is tied to Sway

### [Bar-rs](https://github.com/faervan/bar-rs/tree/main)
Pros:
- Light resource consumption (about 50MB of RAM)

Cons:
- Poor customizability
- Isn't as polished as previously mention projects

Since no currently existing solution fits my need, it's only natural to do the typical developer thing and re-invent the wheel.

That being said, it's probably not the wisest idea to use Rust for a project aiming for minimalism since despite my best efforts (which isn't worth much tbh) to minimize dependencies, the number still ballooned up to 369 (nice) and takes 5 to 10 seconds for a warm build on my laptop (i5 12450H). The memory footprint could be cut down to 1/3rd simply by using something like C and [Clay](https://github.com/nicbarker/clay) but I simply am not that comfortable with debugging segfaults in C.
