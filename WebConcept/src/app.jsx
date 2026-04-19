// App shell — top bar, tool shelf, viewport, right dock, status bar.

const { useState, useEffect, useCallback } = React;

const TOOLS = [
  { id: "select",  key: "V", icon: "Select",   label: "Select" },
  { id: "pan",     key: "H", icon: "Pan",      label: "Pan" },
  { id: "line",    key: "L", icon: "Line",     label: "Line" },
  { id: "rect",    key: "R", icon: "Rect",     label: "Rectangle" },
  { id: "circle",  key: "C", icon: "Circle",   label: "Circle" },
  { id: "arc",     key: "A", icon: "Arc",      label: "Arc" },
  { id: "fillet",  key: "F", icon: "Fillet",   label: "Fillet" },
  { id: "dim",     key: "D", icon: "Dim",      label: "Dimension" },
  { id: "extrude", key: "E", icon: "Extrude",  label: "Extrude" },
];

// Screen → default active tool
const SCREEN_TOOL = {
  sketch:     "line",
  dimension:  "dim",
  constraint: "select",
  extrude:    "extrude",
};

const SCREEN_LABEL = {
  sketch:     "Sketch",
  dimension:  "Dimension",
  constraint: "Constraint",
  extrude:    "Extrude",
};

// HUD hints per screen
const HUD_HINT = {
  sketch:     [["Line",            <><span>Click to set end point ·</span> <span className="k">Shift</span> <span>ortho</span></>]],
  dimension:  [["Dimension",       <><span>Click entity, move pointer, click to place ·</span> <span className="k">Esc</span> <span>cancel</span></>]],
  constraint: [["Select",          <><span>Drag-box or click entities ·</span> <span className="k">Ctrl</span> <span>add</span></>]],
  extrude:    [["Extrude",         <><span>Drag handle or type a distance ·</span> <span className="k">Enter</span> <span>apply</span></>]],
};

function TopBar({ screen, onOpenPalette, onScreen }) {
  return (
    <div className="top-bar">
      <span className="brand">
        <span className="mark"><I.Cube size={14}/></span>
        RONCAD
      </span>
      <span className="vsep"/>
      <div className="selector" title="Active sketch">
        <span className="ico"><I.Sketch size={12}/></span>
        <span>{screen === "extrude" ? "Body 1" : "Sketch 1"}</span>
        <span className="chev"><I.Chevron size={11}/></span>
      </div>
      <button className="icon-btn" title="New sketch"><I.Plus size={13}/></button>
      <span className="vsep"/>
      <button className="icon-btn" title="Save"><I.Save size={13}/></button>
      <button className="icon-btn" title="Undo"><I.Undo size={13}/></button>
      <button className="icon-btn" title="Redo"><I.Redo size={13}/></button>
      <button className="icon-btn" title="Fit view"><I.Fit size={13}/></button>

      <span className="spacer" />

      <div className="cmdk-hint" onClick={onOpenPalette}>
        <I.Search size={11}/>
        <span>Search · run</span>
        <kbd>⌘K</kbd>
      </div>

      <span className="doc-name">Untitled · Bracket</span>

      <div className="mode-chip">
        <span className="dot"/>
        {SCREEN_LABEL[screen]}
      </div>

      <button className="icon-btn" title="Settings"><I.Gear size={13}/></button>
    </div>
  );
}

function ToolShelf({ active, onPick }) {
  return (
    <div className="tool-shelf">
      {TOOLS.map(t => {
        const Glyph = I[t.icon];
        return (
          <button key={t.id}
            className={"tool" + (active === t.id ? " active" : "")}
            onClick={() => onPick(t.id)}>
            <Glyph size={16}/>
            <span className="kbd">{t.key}</span>
            <span className="tool-tip">{t.label} <span className="k">{t.key}</span></span>
          </button>
        );
      })}
      <div className="divider"/>
      <button className="tool" title="Sketch settings">
        <I.Gear size={16}/>
      </button>
    </div>
  );
}

function StatusBar({ screen, tool }) {
  const metrics = {
    sketch: { snap: "Endpoint", x: "30.439", y: "19.862", zoom: "5.00", info: "L 58.310 mm   dX 54.000   dY  3.240   A   3.4°" },
    dimension:  { snap: "Midpoint", x: "55.000", y: "50.000", zoom: "2.00", info: "Click to place dimension" },
    constraint: { snap: null, x: "32.500", y:  "7.000", zoom: "2.00", info: "4 entities selected · 0 DOF remaining" },
    extrude:    { snap: null, x: "21.000", y: "15.000", zoom: "1.30", info: "Distance 15.000 mm · +Z · New body" },
  }[screen];

  const toolLabel = TOOLS.find(t => t.id === tool)?.label || "Select";

  return (
    <div className="status-bar">
      <div className="grp"><span className="k">Mode</span><span className="v accent">{toolLabel}</span></div>
      <div className="sep"/>
      <div className="grp"><span className="k">X</span><span className="v">{metrics.x}</span><span className="k">mm</span></div>
      <div className="grp"><span className="k">Y</span><span className="v">{metrics.y}</span><span className="k">mm</span></div>
      {metrics.snap && <>
        <div className="sep"/>
        <div className="grp"><span className="k">Snap</span><span className="v accent">{metrics.snap}</span></div>
      </>}
      <div className="sep"/>
      <div className="grp"><span className="v" style={{color:"var(--text-mid)"}}>{metrics.info}</span></div>

      <span className="spacer"/>

      <div className="grp"><span className="k">Zoom</span><span className="v">{metrics.zoom}</span><span className="k">px/mm</span></div>
      <div className="sep"/>
      <div className="grp"><span className="k">Units</span><span className="v">mm</span></div>
    </div>
  );
}

function ScreenSwitcher({ screen, onScreen }) {
  const screens = [
    { id: "sketch",     label: "Sketch" },
    { id: "dimension",  label: "Dimension" },
    { id: "constraint", label: "Constraint" },
    { id: "extrude",    label: "Extrude" },
  ];
  return (
    <div className="screens">
      {screens.map((s, i) => (
        <button key={s.id} className={screen === s.id ? "on" : ""} onClick={() => onScreen(s.id)}>
          <span className="num">0{i+1}</span>{s.label}
        </button>
      ))}
    </div>
  );
}

function HudHint({ screen }) {
  const [tag, text] = HUD_HINT[screen][0];
  return (
    <div className="hud-hint">
      <span className="mode-tag">{tag}</span>
      <span className="seg">{text}</span>
      <span className="seg"><span className="k">middle</span> <span>pan</span> · <span className="k">scroll</span> <span>zoom</span></span>
    </div>
  );
}

function ExtrudeHandle() {
  // Inline contextual "dialog-less" extrude control floating in the viewport
  return (
    <div style={{
      position:"absolute", right: 24, top: 70, zIndex: 15,
      background:"var(--bg-elev)", border:"1px solid var(--sep)",
      borderRadius:6, padding:"12px 14px 14px", minWidth: 220,
      boxShadow:"var(--shadow-lift)"
    }}>
      <div style={{display:"flex", alignItems:"center", gap:8, paddingBottom:10, borderBottom:"1px solid var(--sep-soft)"}}>
        <span style={{color:"var(--accent)"}}><I.Extrude size={13}/></span>
        <span style={{fontWeight:500, fontSize:"13px"}}>Extrude</span>
        <span style={{marginLeft:"auto", color:"var(--text-dim)", fontSize:"10.5px", fontFamily:"var(--font-mono)"}}>LIVE</span>
      </div>
      <div style={{display:"grid", gap:8, paddingTop:10}}>
        <div style={{display:"grid", gridTemplateColumns:"70px 1fr", alignItems:"center", gap:8, fontSize:"12px"}}>
          <span style={{color:"var(--text-dim)"}}>Profile</span>
          <span style={{color:"var(--text)", fontFamily:"var(--font-mono)"}}>1 face</span>
        </div>
        <div style={{display:"grid", gridTemplateColumns:"70px 1fr", alignItems:"center", gap:8, fontSize:"12px"}}>
          <span style={{color:"var(--text-dim)"}}>Distance</span>
          <div style={{background:"var(--bg-panel)", border:"1px solid var(--accent-dim)", borderRadius:3, padding:"4px 8px",
                       display:"flex", fontFamily:"var(--font-mono)", color:"var(--accent)"}}>
            15.000<span style={{marginLeft:"auto", color:"var(--text-dim)"}}>mm</span>
          </div>
        </div>
        <div style={{display:"grid", gridTemplateColumns:"70px 1fr", alignItems:"center", gap:8, fontSize:"12px"}}>
          <span style={{color:"var(--text-dim)"}}>Direction</span>
          <div style={{display:"flex", background:"var(--bg-panel)", border:"1px solid var(--sep)", borderRadius:3, overflow:"hidden"}}>
            {["+Z","-Z","Sym"].map((d,i)=>(
              <button key={d} style={{
                flex:1, padding:"3px 0", fontSize:"11px", fontFamily:"var(--font-mono)",
                background: i===0 ? "var(--accent-soft)" : "transparent",
                color: i===0 ? "var(--accent)" : "var(--text-dim)",
                border:0, cursor:"pointer"
              }}>{d}</button>
            ))}
          </div>
        </div>
        <div style={{display:"grid", gridTemplateColumns:"70px 1fr", alignItems:"center", gap:8, fontSize:"12px"}}>
          <span style={{color:"var(--text-dim)"}}>Operation</span>
          <span style={{color:"var(--text)"}}>New body</span>
        </div>
      </div>
      <div style={{display:"flex", gap:6, marginTop:12, justifyContent:"flex-end"}}>
        <button className="btn">Cancel</button>
        <button className="btn primary">Apply</button>
      </div>
    </div>
  );
}

function App() {
  // --- tweakable state ----------------------------------------------------
  const defaultsEl = document.getElementById("tweak-defaults");
  const defaults = JSON.parse(defaultsEl.textContent.replace(/\/\*EDITMODE-[A-Z]+\*\//g, ""));

  const [tweaks, setTweaks] = useState(defaults);
  const [tweaksOpen, setTweaksOpen] = useState(false);
  const [paletteOpen, setPaletteOpen] = useState(false);
  const [tool, setTool] = useState(SCREEN_TOOL[defaults.screen]);

  const setScreen = (s) => {
    setTweaks(t => ({ ...t, screen: s }));
    setTool(SCREEN_TOOL[s]);
  };

  // --- apply accent to body ----------------------------------------------
  useEffect(() => {
    document.body.setAttribute("data-accent", tweaks.accent);
  }, [tweaks.accent]);

  // --- edit mode bridge --------------------------------------------------
  useEffect(() => {
    const onMsg = (e) => {
      const d = e.data || {};
      if (d.type === "__activate_edit_mode") setTweaksOpen(true);
      if (d.type === "__deactivate_edit_mode") setTweaksOpen(false);
    };
    window.addEventListener("message", onMsg);
    window.parent.postMessage({ type: "__edit_mode_available" }, "*");
    return () => window.removeEventListener("message", onMsg);
  }, []);

  const updateTweaks = useCallback((next) => {
    setTweaks(next);
    window.parent.postMessage({ type: "__edit_mode_set_keys", edits: next }, "*");
  }, []);

  // --- keyboard bindings -------------------------------------------------
  useEffect(() => {
    const onKey = (e) => {
      if (e.target.tagName === "INPUT" || e.target.tagName === "TEXTAREA") return;
      const k = e.key.toLowerCase();
      if ((e.metaKey || e.ctrlKey) && k === "k") { e.preventDefault(); setPaletteOpen(v => !v); return; }
      if (paletteOpen) return;
      const t = TOOLS.find(x => x.key.toLowerCase() === k);
      if (t) { setTool(t.id); return; }
      if (k === "1") setScreen("sketch");
      if (k === "2") setScreen("dimension");
      if (k === "3") setScreen("constraint");
      if (k === "4") setScreen("extrude");
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [paletteOpen]);

  // --- command palette handler -------------------------------------------
  const runCommand = useCallback((c) => {
    if (c.action.startsWith("tool:")) {
      const id = c.action.slice(5);
      const toolId = {select:"select", pan:"pan", line:"line", rect:"rect", circle:"circle",
                      arc:"arc", fillet:"fillet", dim:"dim", extrude:"extrude"}[id];
      if (toolId) setTool(toolId);
    } else if (c.action.startsWith("screen:")) {
      setScreen(c.action.slice(7));
    } else if (c.action === "view:grid") {
      updateTweaks({ ...tweaks, grid: tweaks.grid === "lines" ? "dots" : tweaks.grid === "dots" ? "none" : "lines" });
    }
  }, [tweaks, updateTweaks]);

  const screen = tweaks.screen;

  return (
    <div className={"shell" + (tweaks.bold ? " bold" : "")}
         data-screen-label={`0${["sketch","dimension","constraint","extrude"].indexOf(screen)+1} ${SCREEN_LABEL[screen]}`}>
      <TopBar screen={screen} onOpenPalette={() => setPaletteOpen(true)} />
      <ToolShelf active={tool} onPick={setTool} />
      <div className="viewport-wrap">
        <Viewport screen={screen} gridStyle={tweaks.grid} bold={tweaks.bold} />
        <HudHint screen={screen} />
        {screen === "extrude" && <ExtrudeHandle/>}
        {tweaks.bold && <ContextCard screen={screen}/>}
      </div>
      {!tweaks.bold && <RightDock screen={screen} />}
      <StatusBar screen={screen} tool={tool} />

      <ScreenSwitcher screen={screen} onScreen={setScreen} />

      <CommandPalette open={paletteOpen} onClose={() => setPaletteOpen(false)} onCommand={runCommand} />
      {tweaksOpen && <TweaksPanel state={tweaks} onChange={updateTweaks} />}
    </div>
  );
}

ReactDOM.createRoot(document.getElementById("root")).render(<App/>);
