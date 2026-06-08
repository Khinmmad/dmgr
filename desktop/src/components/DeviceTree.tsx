import { useMemo, useState } from "react";
import type { Device } from "../types";
import { STATUS_META } from "../types";

interface Props {
  devices: Device[];
  selectedId: string | null;
  onSelect: (id: string) => void;
}

/** Parent/child hierarchy (USB hubs → devices, PCI bridges → functions). */
export default function DeviceTree({ devices, selectedId, onSelect }: Props) {
  const byId = useMemo(() => {
    const m = new Map<string, Device>();
    for (const d of devices) m.set(d.id, d);
    return m;
  }, [devices]);

  // Roots = no parent, or parent filtered out of the visible set.
  const roots = useMemo(
    () =>
      devices
        .filter((d) => !d.parent || !byId.has(d.parent))
        .sort((a, b) => a.name.localeCompare(b.name)),
    [devices, byId]
  );

  return (
    <div className="cat">
      {roots.map((d) => (
        <TreeNode
          key={d.id}
          node={d}
          byId={byId}
          depth={0}
          selectedId={selectedId}
          onSelect={onSelect}
        />
      ))}
    </div>
  );
}

function TreeNode({
  node,
  byId,
  depth,
  selectedId,
  onSelect,
}: {
  node: Device;
  byId: Map<string, Device>;
  depth: number;
  selectedId: string | null;
  onSelect: (id: string) => void;
}) {
  const [open, setOpen] = useState(depth < 1);
  const kids = node.children
    .map((id) => byId.get(id))
    .filter((d): d is Device => !!d)
    .sort((a, b) => a.name.localeCompare(b.name));
  const hasKids = kids.length > 0;

  return (
    <>
      <button
        className={`dev-item ${selectedId === node.id ? "selected" : ""}`}
        style={{ paddingLeft: 10 + depth * 14 }}
        onClick={() => onSelect(node.id)}
        title={node.name}
      >
        <span
          style={{ width: 12, color: "var(--overlay)" }}
          onClick={(e) => {
            e.stopPropagation();
            if (hasKids) setOpen((o) => !o);
          }}
        >
          {hasKids ? (open ? "▾" : "▸") : ""}
        </span>
        <span className="dot">{STATUS_META[node.status].dot}</span>
        <span className="label">{node.name}</span>
      </button>
      {open &&
        kids.map((k) => (
          <TreeNode
            key={k.id}
            node={k}
            byId={byId}
            depth={depth + 1}
            selectedId={selectedId}
            onSelect={onSelect}
          />
        ))}
    </>
  );
}
