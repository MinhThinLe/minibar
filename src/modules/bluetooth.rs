use std::sync::mpsc::{Sender, TryRecvError, channel};

use dbus::Message;
use dbus::arg::RefArg;
use dbus::blocking::Connection;

use iced::futures::Stream;
use iced::stream;
use iced::widget::{row, text};

use log::{error, warn};

use minibar_derives::{ModuleData, NamedModule};

use crate::dbus::bluetooth_adapter::OrgBluezAdapter1;
use crate::dbus::object_manager::OrgFreedesktopDBusObjectManager;
use crate::dbus::properties::DBusPropertiesChanged;

use super::*;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(1);

#[derive(ModuleData, Debug)]
enum BluetoothEvent {
    NewDevice(BluetoothDevice),
    DeviceConnected(String),
    DeviceDisconnected(String),
    ControllerPoweredOff,
    ControllerPoweredOn,
}

#[derive(Debug, Clone)]
struct BluetoothDevice {
    battery_percentage: Option<u8>,
    icon_name: String,
    path: String,
    connected: bool,
}

struct BluetoothConfig {
    icon_map: HashMap<String, char>,
    format_disabled: Box<str>,
    device_format: Box<str>,
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
            BluetoothEvent::DeviceConnected(device_path) => self.activate_device(device_path),
            BluetoothEvent::DeviceDisconnected(device_path) => self.deactivate_device(device_path),
            BluetoothEvent::ControllerPoweredOff => self.is_powered = false,
            BluetoothEvent::ControllerPoweredOn => self.is_powered = true,
        }
    }

    fn view(&self, _output: &Output) -> Element<'_, BarEvent> {
        if !self.is_powered {
            return text(self.get_text_disabled()).into();
        }

        let devices = self.devices.iter().filter_map(|device| {
            device
                .connected
                .then_some(device.view(&self.config.icon_map, &self.config.device_format))
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

        let icon_map = HashMap::from_iter([
            ("input-keyboard".to_string(), '󰌌'),
            ("input-mouse".to_string(), '󰍽'),
            ("audio-headset".to_string(), '󰋎'),
        ]);

        let config = BluetoothConfig {
            icon_map,
            format_disabled: "off".into(),
            device_format: "{icon} {battery_percentage}".into(),
            spacing: 3,
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
    fn activate_device(&mut self, device_path: &str) {
        let Some(device) = self
            .devices
            .iter_mut()
            .find(|device| device.path == device_path)
        else {
            error!("Requested to activate a non-existent device");
            return;
        };

        device.connected = true;
    }

    fn deactivate_device(&mut self, device_path: &str) {
        let Some(device) = self
            .devices
            .iter_mut()
            .find(|device| device.path == device_path)
        else {
            error!("Requested to deactivate a non-existent device");
            return;
        };

        device.connected = false;
    }

    fn get_text_disabled(&self) -> String {
        self.config.format_disabled.to_string()
    }
}

impl BluetoothDevice {
    fn view(&self, icon_map: &HashMap<String, char>, format: &str) -> Element<'_, BarEvent> {
        const ICON: &str = "{icon}";
        const BATTERY: &str = "{battery_percentage}";

        let icon = icon_map.get(&self.icon_name).copied().unwrap_or_default();

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

        text(
            format
                .replace(BATTERY, &battery_percentage)
                .replace(ICON, &icon.to_string()),
        )
        .into()
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
        let device_proxy = connection.with_proxy("org.bluez", &device.path, DEFAULT_TIMEOUT);
        let sender = sender.clone();
        device_proxy
            .match_signal(
                move |signal: DBusPropertiesChanged,
                      _connection: &Connection,
                      message: &Message| {
                    if let Some(event) = handle_device_signal(&signal, message) {
                        sender.send(event).expect("Unable to send a message");
                        println!("Sent device event");
                    }
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

        let icon = || -> Option<String> {
            let device_info = device.get("org.bluez.Device1")?;
            let icon = device_info.get("Icon")?;
            icon.as_str().map(ToString::to_string)
        }()
        .unwrap_or_default();

        let bluetooth_device = BluetoothDevice {
            battery_percentage,
            path: path.to_string(),
            connected,
            icon_name: icon,
        };

        bluetooth_devices.push(bluetooth_device);
    }

    bluetooth_devices
}

fn handle_device_signal(
    changed_properties: &DBusPropertiesChanged,
    message: &Message,
) -> Option<BluetoothEvent> {
    let path = message.path()?.to_string();
    let is_connected = changed_properties
        .changed_properties
        .get("Connected")?
        .as_u64()
        .map(|num| num == 1);

    if is_connected.unwrap_or_default() {
        Some(BluetoothEvent::DeviceConnected(path))
    } else {
        Some(BluetoothEvent::DeviceDisconnected(path))
    }
}
