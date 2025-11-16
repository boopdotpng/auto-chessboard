use core::time::Duration;

use esp32_nimble::{
    uuid128,
    BLEAdvertisementData,
    BLEDevice,
    NimbleProperties,
    BLECharacteristic,
};
use esp32_nimble::utilities::mutex::Mutex;
use std::sync::Arc;

struct BLE {
    rx_char: Arc<Mutex<BLECharacteristic>>
}

impl BLE {
    pub fn new() -> Self {
        let device = BLEDevice::take();
        let server = device.get_server(); 
        let advertising = device.get_advertising();

        let svc_uuid = uuid128!("6E400001-...");
        let rx_uuid = uuid128!("6E400001-...");

        let service = server.create_service(svc_uuid);

        let rx_char =  {
            let mut svc = service.lock(); 
            svc.create_characteristic(rx_uuid, NimbleProperties::READ | NimbleProperties::WRITE| NimbleProperties::NOTIFY);
        };


        
    }
}