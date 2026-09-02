use std::ffi::OsStr;
use std::rc::Rc;
use std::sync::{Arc, LazyLock, Mutex};
use std::thread::sleep;

use dbus::blocking::{Connection, Proxy};
use dbus::channel::MatchingReceiver;
use dbus::message::{MatchRule, Message};
use dbus::strings::Member;

use iced::core::image::Handle;
use iced::futures::Stream;
use iced::widget::{image, row, svg};
use iced::{Element, Length, Subscription, stream};

use icon_loader::{IconLoader, IconSize};
use log::error;
use minibar_derives::{ModuleData, NamedModule};
use toml::Table;

use crate::dbus::dbus_monitoring::OrgFreedesktopDBusMonitoring;
use crate::dbus::status_notifier_item::OrgKdeStatusNotifierItem;
use crate::dbus::status_notifier_watcher::OrgKdeStatusNotifierWatcher;

use super::*;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(1);
const TRAY_INTERFACE_PATH: &str = "/StatusNotifierWatcher";
const TRAY_INTERFACE_DESTINATION: &str = "org.kde.StatusNotifierWatcher";
const DEFAULT_SPACING: u16 = 3;
const DEFAULT_ICON_SIZE: f32 = 22.0;

static ICON_PROVIDER: LazyLock<IconLoader> =
    LazyLock::new(|| IconLoader::new().unwrap_or_default());

#[derive(Default, Clone)]
enum EventMember {
    #[default]
    NewIcon,
    NewToolTip,
    RegisterStatusNotifierItem,
    StatusNotifierItemRegistered,
    StatusNotifierItemUnregistered,
}

#[derive(ModuleData, Default, Clone)]
struct TrayEvent {
    member: EventMember,
    sender: String,
    args: Vec<String>,
}

struct TrayItem {
    dbus_address: String,
    thumbnail: Handle,
    is_svg: bool,
}

struct TrayConfig {
    spacing: u16,
    icon_size: f32,
}

#[derive(NamedModule)]
pub struct SysTray {
    dbus_connection: Mutex<Connection>,
    tray_items: Vec<TrayItem>,
    style: CommonStyle,
    config: TrayConfig,
}

impl Module for SysTray {
    fn view(&self, _output: &Output) -> Element<'_, BarEvent> {
        let tray_items = self
            .tray_items
            .iter()
            .map(|item| item.view(self.config.icon_size));
        row(tray_items)
            .spacing(self.config.spacing)
            .padding(self.style.padding)
            .into()
    }

    fn update(&mut self, update_data: Arc<dyn ModuleData>) {
        let tray_update = update_data
            .downcast_ref::<TrayEvent>()
            .expect("Who invited bro?");

        match tray_update.member {
            EventMember::NewIcon => self.update_icon(tray_update),
            EventMember::NewToolTip => {
                // TODO: Implement tooltips
            }
            EventMember::StatusNotifierItemRegistered => {
                // TODO: Implement an actual tray server
                // The current implementation only watches for changes and should not function
                // on a system without any systray implementation
            }
            EventMember::RegisterStatusNotifierItem => self.tray_item_added(tray_update),
            EventMember::StatusNotifierItemUnregistered => self.tray_item_removed(tray_update),
        }
    }

    fn subscription(&self) -> Option<Subscription<ModuleUpdate>> {
        Some(Subscription::run(worker))
    }

    fn new_or_default(table: &Table) -> Rc<dyn Module>
    where
        Self: Sized,
    {
        let spacing = get_int(table, "spacing").map_or(DEFAULT_SPACING, |spacing| spacing as u16);
        let icon_size = get_float(table, "preferred_icon_size").unwrap_or(DEFAULT_ICON_SIZE);

        let config = TrayConfig { spacing, icon_size };

        let style = CommonStyle::from(table);

        let connection = Connection::new_session()
            .expect("Could not establish a connection to DBus, is it even running?");

        let proxy = connection.with_proxy(
            TRAY_INTERFACE_DESTINATION,
            TRAY_INTERFACE_PATH,
            DEFAULT_TIMEOUT,
        );

        let tray_items = proxy.get_registered_items().map_or(vec![], |item| {
            item.iter()
                .filter_map(|item| {
                    let (bus_name, bus_path) = item.split_once('/').unwrap_or_default();
                    TrayItem::new(&connection, bus_name, &format!("/{bus_path}"), icon_size)
                })
                .collect()
        });

        Rc::new(Self {
            dbus_connection: Mutex::new(connection),
            tray_items,
            style,
            config,
        })
    }
}

impl SysTray {
    fn update_icon(&mut self, event: &TrayEvent) {
        let connection = self
            .dbus_connection
            .lock()
            .expect("The systray thread shouldn't panic");

        let tray_item = self
            .tray_items
            .iter_mut()
            .find(|item| item.dbus_address == event.sender)
            .expect("What?");
        tray_item.update_thumbnail(&connection, self.config.icon_size as i32);
    }

    fn tray_item_added(&mut self, event: &TrayEvent) {
        let connection = self
            .dbus_connection
            .lock()
            .expect("The systray thread panicked");
        let Some(arg) = event.args.first() else {
            return;
        };
        sleep(Duration::from_millis(500));
        if let Some(tray_item) =
            TrayItem::new(&connection, &event.sender, arg, self.config.icon_size)
        {
            self.tray_items.push(tray_item);
        }
    }

    fn tray_item_removed(&mut self, event: &TrayEvent) {
        let Some(arg) = event.args.first() else {
            return;
        };
        let bus_name = arg.split_once('/').unwrap_or_default().0;
        let Some(position) = self
            .tray_items
            .iter()
            .position(|item| item.dbus_address == bus_name)
        else {
            return;
        };
        self.tray_items.remove(position);
    }
}

impl TrayItem {
    fn new(
        connection: &Connection,
        bus_name: &str,
        bus_path: &str,
        icon_size: f32,
    ) -> Option<Self> {
        let proxy = connection.with_proxy(bus_name, bus_path, DEFAULT_TIMEOUT);
        let dbus_address = bus_name.to_string();
        let thumbnail = TrayItem::get_thumbnail(&proxy, icon_size as i32)?;

        Some(Self {
            dbus_address,
            thumbnail,
            is_svg: false,
        })
    }

    fn get_thumbnail(proxy: &Proxy<'_, &Connection>, preferred_icon_size: i32) -> Option<Handle> {
        let icon_name = proxy.icon_name().unwrap_or_default();
        if let Some(icon_path) =
            ICON_PROVIDER.query_uncached(&icon_name, IconSize::Exact(preferred_icon_size as u16))
        {
            return Some(Handle::from_path(&icon_path));
        }

        if let Some(icon_path) = ICON_PROVIDER.query_uncached(&icon_name, IconSize::Any) {
            return Some(Handle::from_path(&icon_path));
        }

        let Ok(pixmap) = proxy.icon_pixmap() else {
            return None;
        };
        if pixmap.is_empty() {
            return None;
        }

        let image_buffer = pixmap
            .iter()
            .min_by(|array_1, array_2| {
                (array_1.0 - preferred_icon_size)
                    .abs()
                    .cmp(&(array_2.0 - preferred_icon_size).abs())
            })
            .unwrap();

        let rgba = image_buffer
            .2
            .chunks(4)
            .flat_map(|raw_argb| [raw_argb[1], raw_argb[2], raw_argb[3], raw_argb[0]])
            .collect::<Vec<u8>>();

        Some(Handle::from_rgba(
            image_buffer.0.cast_unsigned(),
            image_buffer.1.cast_unsigned(),
            rgba,
        ))
    }

    fn update_thumbnail(&mut self, connection: &Connection, icon_size: i32) {
        let proxy =
            connection.with_proxy(&self.dbus_address, "/StatusNotifierItem", DEFAULT_TIMEOUT);

        if let Some(thumbnail) = Self::get_thumbnail(&proxy, icon_size) {
            self.is_svg = if let Handle::Path(_id, ref path) = thumbnail {
                path.extension() == Some(OsStr::new("svg"))
            } else {
                false
            };
            self.thumbnail = thumbnail;
        }
    }

    fn view(&self, icon_size: f32) -> Element<'_, BarEvent> {
        if let Handle::Path(_id, path) = &self.thumbnail {
            let is_svg = path.extension() == Some(OsStr::new("svg"));
            if is_svg {
                return svg(path)
                    .width(Length::Shrink)
                    .height(Length::Fixed(icon_size))
                    .into();
            }
        }

        image(self.thumbnail.clone())
            .width(Length::Shrink)
            .height(Length::Fixed(icon_size))
            .into()
    }
}

fn request_monitor(
    match_rules: &[&MatchRule],
    timeout: Duration,
) -> Result<Connection, dbus::Error> {
    const MONITORING_BUS_DESTINATION: &str = "org.freedesktop.DBus";
    const MONITORING_BUS_PATH: &str = "/org/freedesktop/DBus";

    let connection = Connection::new_session()?;
    let proxy = connection.with_proxy(MONITORING_BUS_DESTINATION, MONITORING_BUS_PATH, timeout);

    let match_rules: Vec<String> = match_rules.iter().map(|rule| rule.match_str()).collect();
    proxy
        .become_monitor(match_rules.iter().map(String::as_str).collect(), 0u32)
        .map(|_ok_val: ()| connection)
}

fn worker() -> impl Stream<Item = ModuleUpdate> {
    const TRAY_ITEM_PATH: &str = "/StatusNotifierItem";
    const TRAY_ITEM_INTERFACE: &str = "org.kde.StatusNotifierItem";

    const TRAY_INTERFACE: &str = "org.kde.StatusNotifierWatcher";
    const TRAY_PATH: &str = "/StatusNotifierWatcher";

    const TRAY_MODULE_ID: TypeId = TypeId::of::<SysTray>();

    stream::channel(0, async |mut output| {
        let tray_item_update = MatchRule::new()
            .with_path(TRAY_ITEM_PATH)
            .with_interface(TRAY_ITEM_INTERFACE);
        let tray_update = MatchRule::new()
            .with_path(TRAY_PATH)
            .with_interface(TRAY_INTERFACE);

        let Ok(monitor) = request_monitor(&[&tray_update, &tray_item_update], DEFAULT_TIMEOUT)
        else {
            error!("Could not start listening for events on DBus");
            return;
        };

        let (sender, receiver) = std::sync::mpsc::channel();

        let tray_item_sender = sender.clone();
        let tray_event_sender = sender.clone();
        monitor.start_receive(
            tray_item_update,
            Box::new(move |message, _whatever| {
                if let Some(response) = construct_tray_event(&message)
                    && let Err(reason) = tray_item_sender.send(response)
                {
                    error!("[Systray] Communication between threads failed due to {reason}");
                }

                true
            }),
        );

        monitor.start_receive(
            tray_update,
            Box::new(move |message, _whatever| {
                if let Some(response) = construct_tray_event(&message)
                    && let Err(reason) = tray_event_sender.send(response)
                {
                    error!("[Systray] Communication between threads failed due to {reason}");
                }

                true
            }),
        );

        loop {
            monitor
                .process(DEFAULT_TIMEOUT)
                .expect("DBus connection dropped");
            if let Ok(event) = receiver.try_recv() {
                send_data(&mut output, TRAY_MODULE_ID, event).await;
            }
        }
    })
}

impl From<Member<'_>> for EventMember {
    fn from(value: Member<'_>) -> Self {
        match &*value {
            "NewIcon" => Self::NewIcon,
            "NewToolTip" => Self::NewToolTip,
            "RegisterStatusNotifierItem" => Self::RegisterStatusNotifierItem,
            "StatusNotifierItemRegistered" => Self::StatusNotifierItemRegistered,
            "StatusNotifierItemUnregistered" => Self::StatusNotifierItemUnregistered,
            other => {
                error!("Unknown tray event member {other}, assuming NewIcon as fail safe");
                Self::NewIcon
            }
        }
    }
}

fn construct_tray_event(message: &Message) -> Option<TrayEvent> {
    let Some(member) = message.member().map(EventMember::from) else {
        error!("[Systray] DBus message does not contain a member, aborting");
        return None;
    };
    let Some(message_sender) = message.sender() else {
        error!("[Systray] DBus message does not contain a sender, aborting");
        return None;
    };
    let args = match member {
        EventMember::NewToolTip
        | EventMember::NewIcon
        | EventMember::StatusNotifierItemRegistered => vec![],
        EventMember::RegisterStatusNotifierItem | EventMember::StatusNotifierItemUnregistered => {
            vec![message.get1().unwrap()]
        }
    };

    Some(TrayEvent {
        member,
        args,
        sender: message_sender.to_string(),
    })
}
