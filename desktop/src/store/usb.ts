export interface UsbDevice {
  id: string;
  state: string;
}

/** ADB discovery belongs to Phase 5; keep the slice present but empty. */
export interface UsbSlice {
  usb: { devices: UsbDevice[] };
}

export function createUsbSlice(): UsbSlice {
  return { usb: { devices: [] } };
}
