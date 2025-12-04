# what is this? 


# Architecture

### Firmware crate (`src/`)
- `main.rs` bootstraps ESP-IDF, acquires the `Peripherals`, wires up the I²C bus (GPIO2 SDA / GPIO3 SCL), creates the CoreXY motion stack, and starts the event bus. Every hardware resource is configured here before the higher‑level handlers are registered.
- `events.rs` defines the system’s message bus, the `Event` enum consumed across modules, and the `SimpleBleCodec` that translates between BLE frames and internal events. `EventBus::run` forwards every incoming event to each registered handler (BLE, future motion/sense/game actors, etc.).
- `bluetooth.rs` hosts the Nordic UART Service (NUS) peripheral using `esp32-nimble`. It attaches write/notif characteristics to the event bus, decoding any bytes from the phone into `Event`s and encoding outbound `Event`s that map to BLE responses.
- `motion.rs` abstracts the stepper drivers. `Stepper` wraps a STEP/DIR/EN triple, while `CoreXY` coordinates two steppers, the electromagnet, and limit switches to execute `MoveInstruction`s or handle future homing commands.
- `sense.rs` polls four PCF8575 GPIO expanders over I²C to build a 64-bit occupancy mask. It debounces changes into `BoardChange` events that higher layers can interpret as user moves.
- `game.rs` is the future glue between the sensing hardware and the `engine` crate. It will subscribe to board/sense events, keep the authoritative chess state, and emit PGN/FEN updates back onto the event bus.

### Engine crate (`engine/`)
- `lib.rs` exposes the `Engine` API that maintains chess state, validates moves, emits PGN/FEN snapshots, and handles promotions. It tracks pending promotions so hardware/software stay synchronized.
- `board.rs` owns the bitboard-based representation of the chess position, the FEN parser/serializer, and all rule enforcement (move validation, castling/en passant rights, attack detection, etc.).
- `change.rs` converts physical board changes (bitmask diff from the sensor plane) into a `MoveIntent`. It recognizes single-piece moves and castling sequences before the board validates legality.
- `types.rs` hosts all shared data structures (`Move`, `MoveSummary`, `EngineUpdate`, `PromotionRequest`, enums for color, pieces, and castling). These types are what the firmware will eventually exchange over the event bus.
- `util.rs` contains helpers for coordinate↔square conversion and bitboard utilities used throughout the engine.

At runtime the firmware flows as follows:
1. `main` initializes hardware and spawns the `EventBus`.
2. The BLE handler registers against the bus so phone commands immediately arrive as `Event`s.
3. (Planned) Sense and game handlers will observe `BoardChange`s, feed them to `Engine::observe`, and push move summaries back through BLE and motion subsystems.
4. Motion commands (manual `MOTION_GOTO` or future engine-generated `MoveInstruction`s) will be driven through the same bus so all components stay in sync.

## Event Types

The current `Event` enum (see `src/events.rs`) is the source of truth for intra-firmware communication:
- `RequestBattery` / `BatteryReported { percent, charging }` – host requests and board reports charge status.
- `RequestBoardPosition` / `BoardPositionUpdated { fen }` – fetch the current FEN snapshot.
- `RequestPgn` / `SendPgn { pgn }` – fetch the PGN transcript.
- `SetBoardPosition { fen }` – instructs the engine to load an explicit FEN.
- `MovePiece { from, to }` – a raw move command (0–63 square indices) coming from BLE/app.
- `InvalidMove`, `Promotion { piece, square }`, `UndoLastMove` – placeholders for future engine-to-host notifications.
- `MotionCommand(CoreXyCommand)` where `CoreXyCommand` is `Home` or `GotoMM { x, y }`.
- `MotionFinished` – emitted when CoreXY travel completes so BLE/app can unblock.

`BleMessage::try_from(Event)` round-trips only the BLE-visible variants; other events (sense/game/motion internals) stay local to the firmware.

# Bluetooth Protocol

The BLE link uses Nordic UART Service UUIDs (`6E4000…`) and a simple ASCII framing handled by `SimpleBleCodec`:
- Each frame is `CMD` followed by optional payload fields separated by spaces.
- Numeric commands: `BAT <percent> <0|1>`, `MOVE <from> <to>`, `MOTION_GOTO <x_mm> <y_mm>`.
- Zero-argument commands: `REQ_BAT`, `REQ_BOARD`, `REQ_PGN`, `MOTION_HOME`, `MOTION_FINISHED`.
- Length-prefixed string commands (`CMD len:payload`): `BOARD`, `SET_BOARD`, `PGN` where `len` is the byte count of the UTF‑8 payload that follows the colon.
- Direction: phone→board uses the RX characteristic (WRITE/WRITE_NO_RSP); board→phone uses the TX characteristic (NOTIFY).

All BLE messages map one-to-one with `Event` variants so additions only require expanding `BleMessage` plus the codec.

# Hardware Pins Initialized in `main.rs`

| Function           | GPIO | Notes                         |
|--------------------|------|-------------------------------|
| I²C SDA            | 2    | Shared by PCF8575 expanders   |
| I²C SCL            | 3    | Shared by PCF8575 expanders   |
| X stepper STEP     | 11   | `Stepper::new` channel A      |
| X stepper ENABLE   | 12   | Active-low                    |
| X stepper DIR      | 13   | Direction pin                 |
| Y stepper STEP     | 14   | `Stepper::new` channel B      |
| Y stepper ENABLE   | 15   | Active-low                    |
| Y stepper DIR      | 16   | Direction pin                 |
| Electromagnet      | 17   | High = engage magnet          |
| Left limit switch  | 21   | Digital input                 |
| Right limit switch | 38   | Digital input                 |
| Timer (`Timer00`)  | —    | Shared CoreXY timing resource |

The sharing of timer `timer00` with `CoreXY` is configured in `main.rs`, and the `I2cDriver` currently runs at 100 kHz (per comments, intended to increase once hardware is validated).

## Components list

| Component                 | Quantity | Description                               |
|---------------------------|----------|-------------------------------------------|
| PCF8575                   | 4        | 1x16 GPIO expander                        |
| Electromagnet             | 1        | 100 N; 12 V                                |
| Linear hall effect sensor | 64       | DRV5032AJDBZR linear hall effect sensor   |
| SOT23→DIP adapter         | 64       | Breakout to wire hall sensors             |
| Belt                      | N ft     | GT2, 2 mm pitch, 6 mm wide                 |  
| Stepper motor driver      | 2        | TMC2209                                   |
| Stepper motor             | 2        | NEMA 17 pancake                           |
| Belt pulley               | 2        | GT2, 20 teeth, 5 mm bore, 6 mm belt width  |
| Linear ball bearing       | 2        | LM8UU 8 mm ID, 15 mm OD, 24 mm length      |
| Idler                     | 6        | GT2 20T, 5 mm bore, 6 mm wide belt idler   |
| Aluminum rod              | 2        | 300 mm long, 8 mm diameter                 |
| DC-DC buck converter      | 2        | 24V->3.3 for esp32, 5V rail                |

todo! add capactitors and misc. Mosfets here after assembled.

Optionally, light machine oil or lubricant to make it move a little quieter.

And the following 3d printed parts:

## 3D printer models 
