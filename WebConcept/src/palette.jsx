// Cmd-K command palette.

const { useState: useS1, useEffect: useE1, useMemo: useM1 } = React;

const COMMANDS = [
  { group: "Tools", icon: "Select", label: "Select", k: "V", action: "tool:select" },
  { group: "Tools", icon: "Pan", label: "Pan", k: "H", action: "tool:pan" },
  { group: "Tools", icon: "Line", label: "Line", k: "L", action: "tool:line" },
  { group: "Tools", icon: "Rect", label: "Rectangle", k: "R", action: "tool:rect" },
  { group: "Tools", icon: "Circle", label: "Circle", k: "C", action: "tool:circle" },
  { group: "Tools", icon: "Arc", label: "Arc", k: "A", action: "tool:arc" },
  { group: "Tools", icon: "Fillet", label: "Fillet", k: "F", action: "tool:fillet" },
  { group: "Tools", icon: "Dim", label: "Dimension", k: "D", action: "tool:dim", sub: "linear, radial, angular" },
  { group: "Tools", icon: "Extrude", label: "Extrude", k: "E", action: "tool:extrude" },

  { group: "Modify", icon: "Move", label: "Move selection", k: "M", action: "cmd:move" },
  { group: "Modify", icon: "Refresh", label: "Trim entities", action: "cmd:trim" },
  { group: "Modify", icon: "Layers", label: "Offset curve", action: "cmd:offset" },

  { group: "Constraints", icon: "Coincident", label: "Coincident", action: "con:coincident" },
  { group: "Constraints", icon: "Horizontal", label: "Horizontal", action: "con:horizontal" },
  { group: "Constraints", icon: "Vertical", label: "Vertical", action: "con:vertical" },
  { group: "Constraints", icon: "Parallel", label: "Parallel", action: "con:parallel" },
  { group: "Constraints", icon: "Equal", label: "Equal", action: "con:equal" },
  { group: "Constraints", icon: "Tangent", label: "Tangent", action: "con:tangent" },

  { group: "View", icon: "Fit", label: "Fit to view", k: "F6", action: "view:fit" },
  { group: "View", icon: "Grid", label: "Toggle grid", action: "view:grid" },
  { group: "View", icon: "Cube", label: "Switch to Extrude mode", action: "screen:extrude" },

  { group: "Project", icon: "Save", label: "Save project", k: "⌘S", action: "proj:save" },
  { group: "Project", icon: "Plus", label: "New sketch", action: "proj:new-sketch" },
  { group: "Project", icon: "Export", label: "Export STL…", action: "proj:export-stl" },
];

window.CommandPalette = function CommandPalette({ open, onClose, onCommand }) {
  const [q, setQ] = useS1("");
  const [sel, setSel] = useS1(0);
  useE1(() => { if (open) { setQ(""); setSel(0); } }, [open]);

  const filtered = useM1(() => {
    const s = q.trim().toLowerCase();
    const list = s ? COMMANDS.filter(c =>
      c.label.toLowerCase().includes(s) ||
      (c.sub || "").toLowerCase().includes(s) ||
      c.group.toLowerCase().includes(s)
    ) : COMMANDS;
    return list;
  }, [q]);

  const grouped = useM1(() => {
    const g = {};
    filtered.forEach(c => { (g[c.group] = g[c.group] || []).push(c); });
    return g;
  }, [filtered]);

  useE1(() => {
    if (!open) return;
    const onKey = (e) => {
      if (e.key === "Escape") onClose();
      if (e.key === "ArrowDown") { setSel(i => Math.min(i+1, filtered.length-1)); e.preventDefault(); }
      if (e.key === "ArrowUp")   { setSel(i => Math.max(i-1, 0)); e.preventDefault(); }
      if (e.key === "Enter") {
        const c = filtered[sel];
        if (c) { onCommand(c); onClose(); }
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open, sel, filtered, onClose, onCommand]);

  if (!open) return null;

  let idx = -1;
  return (
    <div className="cmdk-overlay" onMouseDown={onClose}>
      <div className="cmdk" onMouseDown={e => e.stopPropagation()}>
        <div className="cmdk-input">
          <span className="pfx">›</span>
          <input autoFocus placeholder="Search tools, commands, constraints…"
            value={q} onChange={e => { setQ(e.target.value); setSel(0); }} />
          <span className="esc">esc</span>
        </div>
        <div className="cmdk-list">
          {Object.entries(grouped).map(([group, items]) => (
            <div key={group}>
              <div className="cmdk-group-title">{group}</div>
              {items.map(c => {
                idx++;
                const Glyph = I[c.icon];
                const active = idx === sel;
                const myIdx = idx;
                return (
                  <div key={c.action} className={"cmdk-item" + (active ? " active" : "")}
                       onMouseEnter={() => setSel(myIdx)}
                       onClick={() => { onCommand(c); onClose(); }}>
                    <span className="ico">{Glyph && <Glyph size={13}/>}</span>
                    <span>{c.label}</span>
                    {c.sub && <span className="sub">{c.sub}</span>}
                    {c.k && <span className="k">{c.k}</span>}
                  </div>
                );
              })}
            </div>
          ))}
          {filtered.length === 0 && (
            <div style={{padding:"24px", textAlign:"center", color:"var(--text-dim)", fontSize:"13px"}}>
              No commands match "{q}"
            </div>
          )}
        </div>
      </div>
    </div>
  );
};
