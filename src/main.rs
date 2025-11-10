mod bluetooth;
mod engine;
mod sense;
use esp_idf_svc::hal::{self, peripheral::Peripheral};

fn main() {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    // one owner, pass from main
    let peripherals = hal::peripherals::Peripherals::take().unwrap();

    let _sense = sense::Sense::new(peripherals.i2c0.into_ref(), 8);

    // is this serial?
    log::info!("Hello, world!");
}
