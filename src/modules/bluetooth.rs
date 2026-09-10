use std::sync::mpsc::{Sender, TryRecvError, channel};

use dbus::arg::RefArg;
use dbus::blocking::Connection;
use dbus::{Message, Path};

use crate::dbus::bluetooth_adapter::OrgBluezAdapter1;
use crate::dbus::object_manager::OrgFreedesktopDBusObjectManager;
use crate::dbus::properties::{DBusPropertiesChanged, OrgFreedesktopDBusProperties};

use super::*;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(1);
const DEFAULT_FORMAT_DISABLED: &str = "BT: off";
const DEFAULT_DEVICE_FORMAT: &str = "{name} {battery_percentage}";
const DEFAULT_FORMAT_IDLE: &str = "BT: idle";
const DEFAULT_SPACING: u16 = 0;

#[derive(ModuleData, Debug)]
enum BluetoothEvent {
    NewDevice(BluetoothDevice),
    DeviceUpdated(BluetoothDevice),
    ControllerPoweredOff,
    ControllerPoweredOn,
}

#[derive(Debug, Clone)]
struct BluetoothDevice {
    battery_percentage: Option<u8>,
    icon_name: Box<str>,
    name: Box<str>,
    path: Box<str>,
    connected: bool,
}

struct BluetoothConfig {
    icon_map: HashMap<String, char>,
    format_disabled: Box<str>,
    format_idle: Box<str>,
    format_device: Box<str>,
    spacing: u16,
}

#[derive(NamedModule)]
pub struct Bluetooth {
    devices: Vec<BluetoothDevice>,
    config: BluetoothConfig,
    style: CommonStyle,
    is_powered: bool,
    id: [ModuleId; 1],
}

impl Module for Bluetooth {
    fn update(&mut self, module_update: ModuleUpdate) {
        let bluetooth_event = module_update
            .1
            .downcast_ref::<BluetoothEvent>()
            .expect("Nah bro get outta here");

        match bluetooth_event {
            BluetoothEvent::NewDevice(device) => self.devices.push(device.clone()),
            BluetoothEvent::DeviceUpdated(device) => self.update_device(device.clone()),
            BluetoothEvent::ControllerPoweredOff => self.is_powered = false,
            BluetoothEvent::ControllerPoweredOn => self.is_powered = true,
        }
    }

    fn view(&self, _output: &Output) -> Element<'_, BarEvent> {
        if !self.is_powered {
            return container(text(self.get_text_disabled()))
                .padding(self.style.padding)
                .into();
        }

        let any_connected = self.devices.iter().any(|device| device.connected);
        if !any_connected {
            return container(text(self.get_text_idle()))
                .padding(self.style.padding)
                .into();
        }

        let devices = self.devices.iter().filter_map(|device| {
            device.connected.then_some(device.view(
                &self.config.icon_map,
                &self.config.format_device,
                self.style,
            ))
        });

        row(devices)
            .spacing(self.config.spacing)
            .padding(self.style.padding)
            .into()
    }

    fn subscription(&self) -> Option<Subscription<ModuleUpdate>> {
        Some(Subscription::run_with(self.id[0], |destination| {
            worker(*destination)
        }))
    }

    fn id(&self) -> &[ModuleId] {
        &self.id
    }

    fn new_or_default(table: &Table) -> Box<dyn Module>
    where
        Self: Sized,
    {
        let id = [module_id_unique()];

        let icon_map = || -> Option<HashMap<String, char>> {
            let icon_map = table.get("icon_map")?;
            let icon_map = icon_map.as_table()?;
            Some(
                (icon_map.iter().filter_map(|(key, value)| {
                    value
                        .as_str()
                        .map(|value| (key.clone(), value.chars().next().unwrap_or_default()))
                }))
                .collect(),
            )
        }()
        .unwrap_or_default();

        let format_disabled = get_str(table, "format_disabled")
            .unwrap_or(DEFAULT_FORMAT_DISABLED)
            .into();
        let format_device = get_str(table, "format_device")
            .unwrap_or(DEFAULT_DEVICE_FORMAT)
            .into();
        let format_idle = get_str(table, "format_idle")
            .unwrap_or(DEFAULT_FORMAT_IDLE)
            .into();
        let spacing = get_int(table, "spacing").map_or(DEFAULT_SPACING, |spacing| spacing as u16);

        let config = BluetoothConfig {
            icon_map,
            format_disabled,
            format_idle,
            format_device,
            spacing,
        };

        let style = CommonStyle::from(table);

        Box::new(Self {
            id,
            is_powered: false,
            config,
            style,
            devices: Vec::new(),
        })
    }
}

impl Bluetooth {
    fn get_device_mut(&mut self, device_path: &str) -> Option<&mut BluetoothDevice> {
        self.devices
            .iter_mut()
            .find(|device| device.path.as_ref() == device_path)
    }

    fn update_device(&mut self, new_device: BluetoothDevice) {
        let Some(device) = self.get_device_mut(&new_device.path) else {
            return;
        };
        *device = new_device;
    }

    fn get_text_disabled(&self) -> String {
        self.config.format_disabled.to_string()
    }

    fn get_text_idle(&self) -> String {
        self.config.format_idle.to_string()
    }
}

impl BluetoothDevice {
    fn view(
        &self,
        icon_map: &HashMap<String, char>,
        format: &str,
        style: CommonStyle,
    ) -> Element<'_, BarEvent> {
        const ICON: &str = "{icon}";
        const BATTERY: &str = "{battery_percentage}";
        const NAME: &str = "{name}";

        let icon = icon_map.get(self.icon_name.as_ref()).copied().unwrap_or_default();

        let battery_percentage = self
            .battery_percentage
            .map(|percent| format!("{percent}%"))
            .unwrap_or_default();

        if format.contains(ICON) && icon == char::default() {
            warn!(
                "Tried to use undefined icon {}. Either define it or remove the {ICON} format",
                self.icon_name
            );
        }

        let mut content = format
            .replace(BATTERY, &battery_percentage)
            .replace(ICON, &icon.to_string())
            .replace(NAME, &self.name);
        content.truncate(content.trim_end().len());

        container(text(content))
            .style(move |_theme| container::Style {
                text_color: style.foreground,
                background: style.get_background(),
                border: style.border,
                ..Default::default()
            })
            .into()
    }

    fn new_from_connection(connection: &Connection, device_path: &Path<'_>) -> Option<Self> {
        let device_proxy = connection.with_proxy("org.bluez", device_path, DEFAULT_TIMEOUT);

        let battery_percentage = device_proxy
            .get("org.bluez.Battery1", "Percentage")
            .ok()
            .map(|prop| {
                prop.as_u64()
                    .expect("Battery percentage should be of type byte") as u8
            });
        let icon_name = device_proxy
            .get("org.bluez.Device1", "Icon")
            .ok()
            .map(|prop| {
                prop.as_str()
                    .expect("Icon should be of type string")
                    .into()
            })?;
        let name = device_proxy
            .get("org.bluez.Device1", "Name")
            .ok()
            .map(|prop| {
                prop.as_str()
                    .expect("Name should be of type string")
                    .into()
            })?;
        let path = device_path.to_string().into();
        let connected = device_proxy
            .get("org.bluez.Device1", "Connected")
            .ok()
            .map(|prop| prop.as_u64().expect("Connected should be of type boolean") == 1)?;

        Some(Self {
            battery_percentage,
            icon_name,
            name,
            path,
            connected,
        })
    }
}

fn worker(module_id: ModuleId) -> impl Stream<Item = ModuleUpdate> {
    stream::channel(0, async move |mut output| {
        let connection = Connection::new_system().unwrap();
        let (sender, receiver) = channel();
        let devices = get_bluetooth_devices(&connection);

        // Newly paired bluetooth devices won't be recognised using this approach but it simplifies
        // things a great deal so why even care?
        setup_device_event_listeners(&connection, &devices, &sender);
        setup_controller_event_listener(&connection, sender);

        for device in devices {
            send_data(&mut output, module_id, BluetoothEvent::NewDevice(device)).await;
        }

        loop {
            connection.process(DEFAULT_TIMEOUT).unwrap();
            match receiver.try_recv() {
                Ok(value) => {
                    send_data(&mut output, module_id, value).await;
                }
                Err(TryRecvError::Empty) => (),
                Err(TryRecvError::Disconnected) => {
                    error!("Bluetooth thread panicked");
                    return;
                }
            }
        }
    })
}

fn setup_controller_event_listener(connection: &Connection, sender: Sender<BluetoothEvent>) {
    fn power_on(sender: &Sender<BluetoothEvent>) {
        sender
            .send(BluetoothEvent::ControllerPoweredOn)
            .expect("Couldn't send a message");
    }

    fn power_off(sender: &Sender<BluetoothEvent>) {
        sender
            .send(BluetoothEvent::ControllerPoweredOff)
            .expect("Couldn't send a message");
    }

    let proxy = connection.with_proxy("org.bluez", "/org/bluez/hci0", DEFAULT_TIMEOUT);
    let Ok(initial_is_powered) = proxy.powered() else {
        error!("Couldn't access the default bluetooth controller");
        return;
    };

    if initial_is_powered {
        power_on(&sender);
    } else {
        power_off(&sender);
    }

    proxy
        .match_signal(
            move |signal: DBusPropertiesChanged, _connection: &Connection, _message: &Message| {
                if let Some(power_state) = signal.changed_properties.get("PowerState")
                    && let Some(power_state) = power_state.as_str()
                {
                    if power_state == "on" {
                        power_on(&sender);
                    }
                    if power_state == "off" {
                        power_off(&sender);
                    }
                    return true;
                }

                if let Some(powered) = signal.changed_properties.get("Powered")
                    && let Some(is_powered) = powered.as_u64().map(|powered| powered == 1)
                {
                    if is_powered {
                        power_on(&sender);
                    } else {
                        power_off(&sender);
                    }
                    return true;
                }
                true
            },
        )
        .expect("Unable to start matching signal");
}

fn setup_device_event_listeners(
    connection: &Connection,
    devices: &[BluetoothDevice],
    sender: &Sender<BluetoothEvent>,
) {
    for device in devices {
        let device_proxy = connection.with_proxy("org.bluez", device.path.as_ref(), DEFAULT_TIMEOUT);
        let sender = sender.clone();
        device_proxy
            .match_signal(
                move |signal: DBusPropertiesChanged, connection: &Connection, message: &Message| {
                    if !should_update_device(&signal) {
                        return true;
                    }
                    let Some(path) = message.path() else {
                        return true;
                    };
                    let Some(updated_device) =
                        BluetoothDevice::new_from_connection(connection, &path)
                    else {
                        return true;
                    };

                    sender
                        .send(BluetoothEvent::DeviceUpdated(updated_device))
                        .expect("Unable to send a message");
                    true
                },
            )
            .expect("Unable to start matching signal");
    }
}

fn get_bluetooth_devices(connection: &Connection) -> Vec<BluetoothDevice> {
    let proxy = connection.with_proxy("org.bluez", "/", DEFAULT_TIMEOUT);
    let Ok(managed_objects) = proxy.get_managed_objects() else {
        return Vec::new();
    };
    let devices: Vec<_> = managed_objects
        .iter()
        .filter(|(_path, device_info)| device_info.contains_key("org.bluez.Device1"))
        .collect();
    let mut bluetooth_devices = Vec::new();

    for (path, device) in devices {
        let battery_percentage = || -> Option<u8> {
            let battery = device.get("org.bluez.Battery1")?;
            let percentage = battery.get("Percentage")?;
            percentage.as_u64().map(|num| num as u8)
        }();

        let connected = || -> Option<bool> {
            let device_info = device.get("org.bluez.Device1")?;
            let connected = device_info.get("Connected")?;
            connected.as_u64().map(|num| num == 1)
        }()
        .unwrap_or_default();

        let get_string = |name| -> Option<Box<str>> {
            let device_info = device.get("org.bluez.Device1")?;
            let icon = device_info.get(name)?;
            icon.as_str().map(std::convert::Into::into)
        };

        let icon_name = get_string("Icon").unwrap_or_default();
        let name = get_string("Name").unwrap_or_default();

        let path = path.to_string().into();

        let bluetooth_device = BluetoothDevice {
            battery_percentage,
            path,
            connected,
            icon_name,
            name,
        };

        bluetooth_devices.push(bluetooth_device);
    }

    bluetooth_devices
}

fn should_update_device(changed_properties: &DBusPropertiesChanged) -> bool {
    // Since bluez for whatever reason won't notify its API consumers about whether any properties
    // in `org.bluez.Battery1` changed and `org.bluez.Device1` isn't a good proxy for
    // `org.bluez.Battery1` since it has some delay. For the time being, this is the only solution
    // that I could think of that doesn't involve polling.
    std::thread::sleep(DEFAULT_TIMEOUT);
    changed_properties.interface == "org.bluez.Device1"
        || changed_properties.interface == "org.bluez.Battery1"
}
