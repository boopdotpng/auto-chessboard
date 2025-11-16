mod bluetooth;
mod motion;
mod events;
mod sense;
use esp_idf_svc::hal::gpio::PinDriver;
use esp_idf_svc::hal::timer::TimerDriver;
use esp_idf_svc::hal::timer::config::Config;
use sense::Sense;
use esp_idf_svc::hal::prelude::*;
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::hal::i2c::{I2cDriver};
use engine::Engine;
use std::thread;

use crate::events::EventBus;
use crate::motion::{CoreXY, Stepper};

fn main() {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take().unwrap();
    let pins = peripherals.pins;

    let sda = pins.gpio2; let scl = pins.gpio3;

    // todo: change freq -- think max is 400
    // for the i2c gpio extenders
    let config = esp_idf_svc::hal::i2c::config::Config::new().baudrate(100.kHz().into());
    let i2c= I2cDriver::new(peripherals.i2c0, sda, scl, &config).unwrap();

    // stepper motor gpio pins
    let step1 = PinDriver::output(pins.gpio11).unwrap();
    let en1 = PinDriver::output(pins.gpio12).unwrap();
    let dir1 = PinDriver::output(pins.gpio13).unwrap();

    let step2 = PinDriver::output(pins.gpio14).unwrap();
    let en2 = PinDriver::output(pins.gpio15).unwrap();
    let dir2 = PinDriver::output(pins.gpio16).unwrap();

    let magnet = PinDriver::output(pins.gpio17).unwrap();
    let left_limit = PinDriver::input(pins.gpio18).unwrap();
    let right_limit = PinDriver::input(pins.gpio19).unwrap();

    let stepper_x = Stepper::new(step1, dir1, en1);
    let stepper_y = Stepper::new(step2, dir2, en2);

    let timer_cfg = Config::new();
    let core_xy_timer = TimerDriver::new(peripherals.timer00, &timer_cfg).unwrap();
    let mut core_xy = CoreXY::new(stepper_x, stepper_y, magnet, left_limit, right_limit, core_xy_timer);
    core_xy.home();

    let _engine = Engine::new();

    let bus = EventBus::new();
    let sense_tx = bus.sender();
    thread::spawn(move || {
        let sense = Sense::new(i2c);
        sense.run(sense_tx);
    });

    let _ = bus.run();
}
