mod bluetooth;
mod motion;
mod events;
mod sense;
use esp_idf_svc::hal::gpio::PinDriver;
use sense::Sense;
use esp_idf_svc::hal::prelude::*;
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::hal::i2c::{I2cDriver, config::Config};
use engine::Engine;

use crate::motion::Stepper;

fn main() -> anyhow::Result<()> {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take().unwrap();
    let pins = peripherals.pins;

    let sda = pins.gpio2; let scl = pins.gpio3;

    // todo: change freq -- think max is 400
    // for the i2c gpio extenders
    let config = Config::new().baudrate(100.kHz().into());
    let i2c= I2cDriver::new(peripherals.i2c0, sda, scl, &config).unwrap();

    // stepper motor gpio pins
    let step1 = PinDriver::output(pins.gpio11).unwrap();
    let en1 = PinDriver::output(pins.gpio12).unwrap();
    let dir1 = PinDriver::output(pins.gpio13).unwrap();

    let step2 = PinDriver::output(pins.gpio14).unwrap();
    let en2 = PinDriver::output(pins.gpio15).unwrap();
    let dir2 = PinDriver::output(pins.gpio16).unwrap();

    let mut sense = Sense::new(i2c);

    let mut m1 = Stepper::new(step1, dir1, en1);
    let mut m2 = Stepper::new(step2, dir2, en2);

    let engine = Engine::new();

    Ok(())
}
