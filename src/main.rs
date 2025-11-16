mod bluetooth;
mod led;
mod motion;
mod events;
mod sense;
mod game;

use std::{thread, time::Duration};

use esp_idf_svc::hal::gpio::PinDriver;
use esp_idf_svc::hal::timer::TimerDriver;
use esp_idf_svc::hal::timer::config::Config;
use esp_idf_svc::hal::prelude::*;
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::hal::i2c::I2cDriver;

use bluetooth::BLE;
use motion::{CoreXY, Stepper};
use events::EventBus;

fn main() {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();
    log::info!("auto-chessboard booting...");

    let peripherals = Peripherals::take().unwrap();
    let pins = peripherals.pins;
    log::info!("peripherals acquired, configuring I2C and motion hardware");

    let sda = pins.gpio2;
    let scl = pins.gpio3;

    // todo: change freq -- think max is 400
    // for the i2c gpio extenders
    let config = esp_idf_svc::hal::i2c::config::Config::new().baudrate(100.kHz().into());
    let _i2c = I2cDriver::new(peripherals.i2c0, sda, scl, &config).unwrap();
    log::info!("i2c0 configured on SDA GPIO2 / SCL GPIO3");

    // stepper motor gpio pins
    let step1 = PinDriver::output(pins.gpio11).unwrap();
    let en1 = PinDriver::output(pins.gpio12).unwrap();
    let dir1 = PinDriver::output(pins.gpio13).unwrap();

    let step2 = PinDriver::output(pins.gpio14).unwrap();
    let en2 = PinDriver::output(pins.gpio15).unwrap();
    let dir2 = PinDriver::output(pins.gpio16).unwrap();

    let magnet = PinDriver::output(pins.gpio17).unwrap();
    let left_limit = PinDriver::input(pins.gpio21).unwrap();
    let right_limit = PinDriver::input(pins.gpio38).unwrap();

    let stepper_x = Stepper::new(step1, dir1, en1);
    let stepper_y = Stepper::new(step2, dir2, en2);
    log::info!("stepper GPIO configured (step: 11/14, dir: 13/16, enable: 12/15, magnet: 17, limits: 18/19)");

    let timer_cfg = Config::new();
    let core_xy_timer = TimerDriver::new(peripherals.timer00, &timer_cfg).unwrap();
    let _core_xy = CoreXY::new(stepper_x, stepper_y, magnet, left_limit, right_limit, core_xy_timer);
    log::warn!("CoreXY homing temporarily disabled (limit switches not verified)");

    let mut event_bus = EventBus::new();
    log::info!("event bus online, initializing BLE");

    let bluetooth = BLE::new(event_bus.sender());
    event_bus.register_handler(Box::new(bluetooth));
    log::info!("BLE advertising started, entering event loop");

    let _heartbeat = thread::spawn(|| loop {
        log::info!("heartbeat: main loop alive");
        thread::sleep(Duration::from_secs(5));
    });

    if let Err(err) = event_bus.run() {
        log::error!("event bus stopped: {err:?}");
    }
}
