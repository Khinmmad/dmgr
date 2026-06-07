import { useMemo } from "react";
import type { View } from "../App";
import type { Bus, Device } from "../types";
import { BUS_META, STATUS_META } from "../types";

interface Props {
  view: View;
  devices: Device[];
  total: number;
  selectedId: string | null;
  onSelect: (id: string) => void;
}

const BUS_ORDER: Bus[] = [
  "Drm",
  "Net",
  "Audio",
  "Usb",
  "Block",
  "Input",
  "Hid",
  "Pci",
  "Power",
  "Tty",
  "IoMMU",
  "Unknown",
];

export default function Sidebar({ devices, selectedId, onSelect }: Props) {
  const groups = useMemo(() => {
    const map = new Map<Bus, Device[]>();
    for (const d of devices) {
      const arr = map.get(d.bus) ?? [];
      arr.push(d);
      map.set(d.bus, arr);
    }
    return BUS_ORDER.filter((b) => map.has(b)).map((b) => ({
      bus: b,
      items: map.get(b)!,
    }));
  }, [devices]);

  return (
    <nav className="sidebar">
      {groups.length === 0 && (
        <div className="empty" style={{ padding: "30px 10px" }}>
          No devices match.
        </div>
      )}
      {groups.map(({ bus, items }) => (
        <div className="cat" key={bus}>
          <div className="cat-header">
            <span>{BUS_META[bus].icon}</span>
            <span>{BUS_META[bus].label}</span>
            <span className="count">{items.length}</span>
          </div>
          {items.map((d) => (
            <button
              key={d.id}
              className={`dev-item ${selectedId === d.id ? "selected" : ""}`}
              onClick={() => onSelect(d.id)}
              title={d.name}
            >
              <span className="dot">{STATUS_META[d.status].dot}</span>
              <span className="label">{d.name}</span>
            </button>
          ))}
        </div>
      ))}
    </nav>
  );
}
