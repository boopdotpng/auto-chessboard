#!/usr/bin/env python3
"""
Scan for Bluetooth Low Energy (BLE) devices and enumerate
their services, characteristics, and descriptors.

Requires:
    pip install bleak
"""

import asyncio
from bleak import BleakScanner, BleakClient
from bleak.exc import BleakError

SCAN_TIMEOUT = 5.0  # seconds


async def scan_devices():
    """Scan for nearby BLE devices and print a numbered list."""
    print(f"Scanning for BLE devices for {SCAN_TIMEOUT} seconds...")
    devices = await BleakScanner.discover(timeout=SCAN_TIMEOUT)
    if not devices:
        print("No BLE devices found.")
        return []

    print("\nFound devices:")
    for idx, d in enumerate(devices):
        name = d.name or "Unknown"
        # RSSI / metadata may not always be available or may be deprecated,
        # so only show what we can safely access.
        print(f"[{idx}] {name} ({d.address})")

    return devices


async def explore_device(device):
    """Connect to a device and enumerate its services/characteristics."""
    print(f"\nConnecting to {device.name or 'Unknown'} ({device.address}) ...")

    try:
        async with BleakClient(device) as client:
            if not client.is_connected:
                print("Failed to connect.")
                return

            print("Connected.")
            print("Discovering services...")
            services = await client.get_services()  # populates client.services

            print("\n=== GATT Services / Characteristics / Descriptors ===")
            for service in services:
                print(f"\n[Service] {service.uuid}  ({service.description})")

                for char in service.characteristics:
                    props = ",".join(char.properties) if char.properties else ""
                    print(f"  [Characteristic] {char.uuid}  ({char.description})")
                    print(f"    Handle:      {char.handle}")
                    print(f"    Properties:  {props or 'None'}")

                    if char.descriptors:
                        print("    Descriptors:")
                        for desc in char.descriptors:
                            print(
                                f"      Handle {desc.handle}: "
                                f"{desc.uuid}  ({desc.description})"
                            )

            print("\nDone.")

    except BleakError as e:
        print(f"Bleak error: {e}")
    except Exception as e:
        print(f"Unexpected error: {e}")


async def main():
    devices = await scan_devices()
    if not devices:
        return

    # Ask user which device to connect to
    while True:
        choice = input(
            "\nEnter device index to inspect (or 'q' to quit): "
        ).strip()
        if choice.lower() in {"q", "quit", "exit"}:
            return

        try:
            idx = int(choice)
        except ValueError:
            print("Please enter a valid number.")
            continue

        if 0 <= idx < len(devices):
            await explore_device(devices[idx])
            break
        else:
            print(f"Index out of range (0–{len(devices) - 1}).")


if __name__ == "__main__":
    asyncio.run(main())
