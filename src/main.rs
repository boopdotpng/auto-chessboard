mod bluetooth;
mod engine;
mod motion;
mod sense;
use sense::Sense;
use esp_idf_svc::hal::prelude::*;
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::hal::i2c::{I2cDriver, config::Config};

fn main() {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    // one owner, pass from main
    let peripherals = Peripherals::take().unwrap();
    let pins = peripherals.pins;

    let sda = pins.gpio2; let scl = pins.gpio3;

    // todo: change freq -- think max is 400
    let config = Config::new().baudrate(100.kHz().into());
    let i2c= I2cDriver::new(peripherals.i2c0, sda, scl, &config).unwrap();

    let mut sense = Sense::new(i2c);

    // is this serial?
    log::info!("Hello, world!");
}
