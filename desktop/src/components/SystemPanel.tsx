import { useCallback, useEffect, useState } from "react";
import type { Notify } from "../App";
import { api } from "../api";
import type { SystemInfo } from "../types";

interface Props {
  notify: Notify;
}

export default function SystemPanel({ notify }: Props) {
  const [info, setInfo] = useState<SystemInfo | null>(null);

  const load = useCallback(async () => {
    try {
      setInfo(await api.systemInfo());
    } catch (e) {
      notify(String(e), "err");
    }
  }, [notify]);

  useEffect(() => {
    load();
    const t = window.setInterval(load, 4000);
    return () => window.clearInterval(t);
  }, [load]);

  if (!info) return <div className="spinner">Reading system info…</div>;

  const memPct =
    info.mem_total_mb > 0 ? Math.round((info.mem_used_mb / info.mem_total_mb) * 100) : 0;

  return (
    <div>
      <div className="row between">
        <div>
          <div className="panel-title">🖥 System</div>
          <div className="panel-sub">Live overview of this machine.</div>
        </div>
        <button className="iconbtn" onClick={load} title="Refresh">
          ⟳
        </button>
      </div>

      <div className="section-h">Memory</div>
      <div className="card">
        <div className="row between" style={{ marginBottom: 8 }}>
          <span className="set-label">RAM</span>
          <span className="panel-sub" style={{ margin: 0 }}>
            {fmtGib(info.mem_used_mb)} / {fmtGib(info.mem_total_mb)} GiB · {memPct}%
          </span>
        </div>
        <div className="meter">
          <div
            className="meter-fill"
            style={{ width: `${memPct}%`, background: memPct > 85 ? "var(--red)" : "var(--accent)" }}
          />
        </div>
      </div>

      <div className="section-h">Details</div>
      <div className="card">
        <table className="prop-table">
          <tbody>
            <Row k="Hostname" v={info.hostname} />
            <Row k="CPU" v={info.cpu_model} />
            <Row k="Cores / threads" v={info.cpu_cores ? String(info.cpu_cores) : ""} />
            <Row k="Kernel" v={info.kernel} />
            <Row k="Architecture" v={info.arch} />
            <Row k="Uptime" v={info.uptime} />
            <Row k="Load average" v={info.load_avg} />
          </tbody>
        </table>
      </div>
    </div>
  );
}

function Row({ k, v }: { k: string; v: string }) {
  if (!v) return null;
  return (
    <tr>
      <td className="k">{k}</td>
      <td className="v">{v}</td>
    </tr>
  );
}

function fmtGib(mb: number): string {
  return (mb / 1024).toFixed(1);
}
