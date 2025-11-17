#!/usr/bin/env python3
"""
Scan for Bluetooth Low Energy (BLE) devices, find a device named
"auto-chessboard", connect to it, and enumerate its services,
characteristics, and descriptors.

Requires:
    pip install bleak
"""

import asyncio
import sys
from bleak import BleakScanner, BleakClient
from bleak.exc import BleakError

SCAN_TIMEOUT = 5.0  # seconds
TARGET_NAME = "auto-chessboard"


async def find_auto_chessboard():
    """Scan for BLE devices and return the one named TARGET_NAME."""
    print(f"Scanning for BLE devices for {SCAN_TIMEOUT} seconds...")
    devices = await BleakScanner.discover(timeout=SCAN_TIMEOUT)

    if not devices:
        print("No BLE devices found.")
        return None

    print("\nFound devices:")
    target_device = None
    for d in devices:
        name = d.name or "Unknown"
        # print(f"- {name} ({d.address})")
        if name == TARGET_NAME:
            target_device = d

    if target_device is None:
        print(f"\nError: No device named '{TARGET_NAME}' found.")
        return None

    print(f"\nSelected device: {target_device.name or 'Unknown'} "
          f"({target_device.address})")
    return target_device


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

            # Newer Bleak: services are discovered on connect and exposed via
            # the `services` property.
            services = getattr(client, "services", None)

            # For safety: if services is None for some reason, try old API
            # if it exists.
            if services is None and hasattr(client, "get_services"):
                services = await client.get_services()

            if services is None:
                print("No services found (service discovery failed).")
                return

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
    device = await find_auto_chessboard()
    if device is None:
        # Non-zero exit code to signal error
        sys.exit(1)

    await explore_device(device)


if __name__ == "__main__":
    asyncio.run(main())
