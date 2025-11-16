use std::sync::Arc;
use esp32_nimble::{
    uuid128,
    BLEAdvertisementData,
    BLEDevice,
    NimbleProperties,
    BLECharacteristic,
};
use esp32_nimble::utilities::mutex::Mutex;

use crate::events::{EventHandler, EventSender};

const SVC_UUID: &str = "6E400001-B5A3-F393-E0A9-E50E24DCCA9E";
const RX_UUID:  &str = "6E400002-B5A3-F393-E0A9-E50E24DCCA9E"; // phone -> board (WRITE/WRITE_NO_RSP)
const TX_UUID:  &str = "6E400003-B5A3-F393-E0A9-E50E24DCCA9E"; // board -> phone (NOTIFY)

pub struct BLE {
    rx_char: Arc<Mutex<BLECharacteristic>>, // phone -> board
    tx_char: Arc<Mutex<BLECharacteristic>>, // board -> phone
    sender: EventSender,
}

impl BLE {
    pub fn new(sender: EventSender) -> Self {
        let device = BLEDevice::take();
        let server = device.get_server(); 
        let advertising = device.get_advertising();

        let svc_uuid = uuid128!(SVC_UUID);
        let rx_uuid = uuid128!(RX_UUID);
        let tx_uuid = uuid128!(TX_UUID);

        let service = server.create_service(svc_uuid);

        let rx_char = service
            .lock()
            .create_characteristic(
                rx_uuid,
                NimbleProperties::WRITE | NimbleProperties::WRITE_NO_RSP,
            );

        let tx_char = service
            .lock()
            .create_characteristic(
                tx_uuid,
                NimbleProperties::READ | NimbleProperties::NOTIFY,
            );

        tx_char.lock().set_value(b"");

        server.start().unwrap();

        advertising
            .lock()
            .set_data(
                BLEAdvertisementData::new()
                    .name("auto-chessboard")
                    .add_service_uuid(svc_uuid),
            )
            .unwrap();

        advertising.lock().start().unwrap();

        BLE {
            rx_char, 
            tx_char, 
            sender,
        }

    }
}

impl EventHandler for BLE {
    fn handle(&mut self, evt: &crate::events::Event) -> anyhow::Result<()> {
       match evt {
        _ => {}
       }
       Ok(()) 
    }
}