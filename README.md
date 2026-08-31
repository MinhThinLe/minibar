# Minibar (name WIP)

A status bar for wayland that's meant to do less (with less, duh)

# Features

Currently, there's support for the following modules
- Battery
- Clock
- Cpu
- Memory
- Systray
- Temperature
- Workspaces (Only supports Niri atm)

Support for the following modules is planned
- Bluetooth
- Group
- Audio (pipewire and maybe ALSA too)
- Backlight
- Idle inhibitor

# Configuration

Since the project is still in its infancy stages, the configuration format hasn't been finalized yet. Though there is an [example config](./examples/configs/test-config.toml)

# Motivation

I wanted a status bar that has all the following features
- Is just a bar, not an entire desktop shell
- Good multi-monitor support (doesn't crash on monitor hotplug)
- Use as little system resource as possible (so I can LARP)

I have checked out the following projects

**Waybar**\
Pros:
- Is nothing more than a bar
- OK-ish resource consumption (roughly 100MB of RAM)
Cons:
- It sometimes crashes on monitor hot-plug. Not often enough to be easily debug-able, just enough to be annoying

**Noctalia**\
Pros:
- Hot-plugging monitors never crashes the shell
- Just as light as the services it's meant to replace (Waybar + Swayidle + Hyprlock + SwayOSD + Mako + Rofi, etc)
Cons:
- Does too much

**Ironbar**\
Pros:
- Good customizability
Cons:
- Just as heavy as Waybar since it's written with GTK, might as well use Waybar at that point

**Dwl's bar**\
Pros:
- Is just a bar, nothing more, nothing less
- *Tiny* footprint (~1 or 2 MB of RAM, can't be sure since it's a dwl patch rather than a separate program)
Cons:
- Is tied to dwl

**Swaybar**\
Pros:
- Small footprint (17MB of RAM)
- Also have other things I need (like a systray)
Cons:
- Is tied to Sway

**Bar-rs**\
Pros:
- Light resource consumption (about 50MB of RAM)
Cons:
- Poor customizability
- Isn't as polished as previously mention projects
