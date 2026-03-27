#![windows_subsystem = "windows"]

use std::time::{Duration, Instant};

use battery::{Manager, State};
use notify_rust::{Notification, Timeout};
use tray_icon::{
    TrayIconBuilder,
    menu::{Menu, MenuEvent, MenuId, MenuItem},
};
use winit::event_loop::{ControlFlow, EventLoop};

const TRIGGER_DELAY: Duration = Duration::from_secs(2);
const SUPPRESS_PERCENT: f32 = 0.90;

fn main() {
    let manager = Manager::new().unwrap();
    let mut battery = match manager.batteries().unwrap().next() {
        Some(Ok(battery)) => battery,
        Some(Err(err)) => {
            eprintln!("unable to access battery information: {err}");
            return;
        }
        None => {
            eprintln!("unable to find a battery");
            return;
        }
    };

    let mut unplugged_time = None;
    let mut sent = true; // don't notify immediately after launching

    let tray_menu = Menu::new();
    tray_menu
        .append_items(&[
            &MenuItem::new("Battery Notifier", false, None),
            &MenuItem::with_id("notify", "Test Notification", true, None),
            &MenuItem::with_id("exit", "Exit", true, None),
        ])
        .unwrap();
    let _tray_icon = TrayIconBuilder::new()
        .with_tooltip("Battery Notifier")
        .with_menu(Box::new(tray_menu))
        .build()
        .unwrap();

    let event_loop = EventLoop::new().unwrap();

    event_loop
        .run(move |_event, _loop| {
            _loop.set_control_flow(ControlFlow::WaitUntil(
                Instant::now() + Duration::from_secs(1),
            ));

            let _ = manager.refresh(&mut battery);

            if !matches!(battery.state(), State::Charging) {
                unplugged_time.get_or_insert_with(Instant::now);
            } else {
                unplugged_time = None;
                sent = false;
            }

            if let Some(time) = unplugged_time
                && !sent
                && time.elapsed() > TRIGGER_DELAY
            {
                if battery.state_of_charge().value < SUPPRESS_PERCENT {
                    notify();
                }
                sent = true;
            }

            if let Ok(event) = MenuEvent::receiver().try_recv() {
                match event.id {
                    MenuId(id) if &id == "exit" => _loop.exit(),
                    MenuId(id) if &id == "notify" => {
                        notify();
                    }
                    _ => println!("{:?}", event),
                }
            }
        })
        .unwrap();

    println!("exiting...")
}

fn notify() {
    Notification::new()
        .summary("Device Unplugged")
        .body(&format!(
            "The charger was disconnected while below {}% charge",
            (SUPPRESS_PERCENT * 100.0).round()
        ))
        .timeout(Timeout::Milliseconds(6000))
        .show()
        .unwrap();
}
