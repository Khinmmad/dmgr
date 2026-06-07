import { useMemo, useState } from "react";
import type { Notify } from "../App";
import { api } from "../api";
import type { Device } from "../types";

interface Props {
  device: Device;
  notify: Notify;
  onChanged: () => void;
}

export default function PropertyTable({ device, notify, onChanged }: Props) {
  const editable = useMemo(
    () => new Set(device.editable_properties),
    [device.editable_properties]
  );

  const entries = useMemo(
    () =>
      Object.entries(device.properties).sort(([a], [b]) => {
        const ea = editable.has(a) ? 0 : 1;
        const eb = editable.has(b) ? 0 : 1;
        return ea - eb || a.localeCompare(b);
      }),
    [device.properties, editable]
  );

  if (entries.length === 0) {
    return <div className="card panel-sub">No properties exposed by this device.</div>;
  }

  return (
    <div className="card">
      <table className="prop-table">
        <tbody>
          {entries.map(([key, value]) => (
            <tr key={key}>
              <td className="k">{key}</td>
              <td className="v">
                {editable.has(key) ? (
                  <EditableValue
                    path={device.path}
                    name={key}
                    initial={value}
                    notify={notify}
                    onChanged={onChanged}
                  />
                ) : (
                  value
                )}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

function EditableValue({
  path,
  name,
  initial,
  notify,
  onChanged,
}: {
  path: string;
  name: string;
  initial: string;
  notify: Notify;
  onChanged: () => void;
}) {
  const [val, setVal] = useState(initial);
  const [busy, setBusy] = useState(false);
  const dirty = val !== initial;

  const save = async () => {
    setBusy(true);
    try {
      await api.setProperty(path, name, val);
      notify(`${name} = ${val}`, "ok");
      onChanged();
    } catch (e) {
      notify(String(e), "err");
      setVal(initial);
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="row" style={{ gap: 8 }}>
      <input
        className="prop-input"
        value={val}
        disabled={busy}
        onChange={(e) => setVal(e.target.value)}
        onKeyDown={(e) => e.key === "Enter" && dirty && save()}
      />
      <button className="btn" disabled={!dirty || busy} onClick={save}>
        Save
      </button>
    </div>
  );
}
