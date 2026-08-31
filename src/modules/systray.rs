use std::ffi::OsStr;
use std::rc::Rc;
use std::sync::{Arc, LazyLock, Mutex};

use dbus::blocking::Connection;
use dbus::channel::MatchingReceiver;
use dbus::message::{MatchRule, Message};
use dbus::strings::Member;

use iced::core::image::Handle;
use iced::futures::Stream;
use iced::widget::{image, row, svg};
use iced::{Element, Length, Subscription, stream};

use icon_loader::{IconLoader, IconSize};
use log::{error, info};
use minibar_derives::{ModuleData, NamedModule};
use toml::Table;

use super::*;
use crate::dbus::dbus_monitoring::OrgFreedesktopDBusMonitoring;
use crate::dbus::status_notifier_item::OrgKdeStatusNotifierItem;
use crate::dbus::status_notifier_watcher::OrgKdeStatusNotifierWatcher;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(1);
const TRAY_INTERFACE_PATH: &str = "/StatusNotifierWatcher";
const TRAY_INTERFACE_DESTINATION: &str = "org.kde.StatusNotifierWatcher";

static ICON_PROVIDER: LazyLock<IconLoader> =
    LazyLock::new(|| IconLoader::new().unwrap_or_default());

#[derive(Default, Clone)]
enum EventMember {
    #[default]
    NewIcon,
    NewToolTip,
}

#[derive(ModuleData, Default, Clone)]
struct TrayEvent {
    member: EventMember,
    sender: String,
}

struct TrayItem {
    dbus_address: String,
    thumbnail: Handle,
    is_svg: bool,
}

#[derive(NamedModule)]
pub struct SysTray {
    dbus_connection: Mutex<Connection>,
    tray_items: Vec<TrayItem>,
}

impl Module for SysTray {
    fn view(&self, _output: &Output) -> Element<'_, BarEvent> {
        row(self.tray_items.iter().map(TrayItem::view)).into()

    }

    fn update(&mut self, update_data: Arc<dyn ModuleData>) {
        let tray_update = update_data
            .downcast_ref::<TrayEvent>()
            .expect("Who invited bro?");

        info!("[Systray] TODO: I should update {}", tray_update.sender);
        let tray_item = self.tray_items.iter_mut().find(|item| item.dbus_address == tray_update.sender).expect("Unimplemented");
        let connection = self.dbus_connection.lock().unwrap();
        tray_item.update_thumbnail(&connection);
    }

    fn subscription(&self) -> Option<Subscription<ModuleUpdate>> {
        Some(Subscription::run(worker))
    }

    fn new_or_default(table: &Table) -> Rc<dyn Module>
    where
        Self: Sized,
    {
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
                    TrayItem::initialize(&connection, item.split_once('/').unwrap_or_default().0)
                })
                .collect()
        });

        Rc::new(Self {
            dbus_connection: Mutex::new(connection),
            tray_items,
        })
    }
}

impl TrayItem {
    fn initialize(connection: &Connection, bus_name: &str) -> Option<Self> {
        let proxy = connection.with_proxy(bus_name, "/StatusNotifierItem", DEFAULT_TIMEOUT);

        let icon_name = proxy.icon_name().unwrap_or_default();
        let (thumbnail, is_svg) = if let Some(icon_path) = ICON_PROVIDER.query_uncached(&icon_name, IconSize::Any) {
            (Handle::from_path(&icon_path), &icon_path.extension() == &Some(OsStr::new("svg")))
        } else {
            let pixmap = proxy.icon_pixmap().map_or(vec![], |icon| icon[0].2.clone());
            (Handle::from_bytes(pixmap), false)
        };

        Some(Self {
            dbus_address: bus_name.to_string(),
            thumbnail,
            is_svg,
        })
    }

    fn update_thumbnail(&mut self, connection: &Connection) {
        let proxy = connection.with_proxy(&self.dbus_address, "/StatusNotifierItem", DEFAULT_TIMEOUT);
        let icon_name = proxy.icon_name().unwrap_or_default();
        let (thumbnail, is_svg) = if let Some(icon_path) = ICON_PROVIDER.query_uncached(&icon_name, IconSize::Any) {
            (Handle::from_path(&icon_path), &icon_path.extension() == &Some(OsStr::new("svg")))
        } else {
            let pixmap = proxy.icon_pixmap().map_or(vec![], |icon| icon[0].2.clone());
            (Handle::from_bytes(pixmap), false)
        };

        self.thumbnail = thumbnail;
        self.is_svg = is_svg;
    }

    fn view(&self) -> Element<'_, BarEvent> {
        if self.is_svg {
            let Handle::Path(_id, path) = &self.thumbnail else {
                panic!("Mislabled TrayItem as SVG when it isn't");
            };
            return svg(path).width(Length::Shrink).into();
        }

        image(self.thumbnail.clone()).width(Length::Shrink).into()
    }
}

fn request_monitor(match_rule: MatchRule, timeout: Duration) -> Result<Connection, dbus::Error> {
    const MONITORING_BUS_DESTINATION: &str = "org.freedesktop.DBus";
    const MONITORING_BUS_PATH: &str = "/org/freedesktop/DBus";

    let connection = Connection::new_session()?;
    let proxy = connection.with_proxy(MONITORING_BUS_DESTINATION, MONITORING_BUS_PATH, timeout);

    proxy
        .become_monitor(vec![&match_rule.match_str()], 0u32)
        .map(|_ok_val: ()| connection)
}

fn worker() -> impl Stream<Item = ModuleUpdate> {
    const TRAY_INTERFACE: &str = "org.kde.StatusNotifierItem";
    const TRAY_PATH: &str = "/StatusNotifierItem";
    const TRAY_MODULE_ID: TypeId = TypeId::of::<SysTray>();

    stream::channel(0, async |mut output| {
        let monitoring_rule = MatchRule::new()
            .with_path(TRAY_PATH)
            .with_interface(TRAY_INTERFACE);

        let Ok(monitor) = request_monitor(monitoring_rule.clone(), DEFAULT_TIMEOUT) else {
            error!("Could not start listening for events on DBus");
            return;
        };

        let (sender, receiver) = std::sync::mpsc::channel();

        monitor.start_receive(
            monitoring_rule,
            Box::new(move |message, _whatever| {
                if let Some(response) = construct_response(&message)
                    && let Err(reason) = sender.send(response)
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
            other => {
                error!("Unknown tray event member {other}, assuming NewIcon as fail safe");
                Self::NewIcon
            }
        }
    }
}

fn construct_response(message: &Message) -> Option<TrayEvent> {
    let Some(member) = message.member().map(EventMember::from) else {
        error!("[Systray] DBus message does not contain a member, aborting");
        return None;
    };
    let Some(message_sender) = message.sender() else {
        error!("[Systray] DBus message does not contain a sender, aborting");
        return None;
    };

    Some(TrayEvent {
        member,
        sender: message_sender.to_string(),
    })
}
