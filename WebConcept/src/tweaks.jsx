// Tweaks panel — visible when edit-mode is on.

const ACCENTS = [
  { id: "blue",   color: "#4FA3F7", label: "Blue" },
  { id: "amber",  color: "#E2A246", label: "Amber" },
  { id: "green",  color: "#6BD39A", label: "Green" },
  { id: "violet", color: "#9E8BF5", label: "Violet" },
];

window.TweaksPanel = function TweaksPanel({ state, onChange }) {
  const set = (k, v) => onChange({ ...state, [k]: v });
  return (
    <div className="tweaks fade-in">
      <div className="head"><span className="dot"/> Tweaks</div>
      <div className="body">

        <div className="row">
          <span className="lbl">Accent</span>
          <div className="swatches">
            {ACCENTS.map(a => (
              <div key={a.id}
                   className={"swatch" + (state.accent === a.id ? " on" : "")}
                   style={{ background: a.color }}
                   title={a.label}
                   onClick={() => set("accent", a.id)}>
                {state.accent === a.id ? "✓" : ""}
              </div>
            ))}
          </div>
        </div>

        <div className="row">
          <span className="lbl">Grid</span>
          <div className="seg">
            {["lines","dots","none"].map(g => (
              <button key={g} className={state.grid === g ? "on" : ""}
                onClick={() => set("grid", g)}>{g[0].toUpperCase()+g.slice(1)}</button>
            ))}
          </div>
        </div>

        <div className="row">
          <div className={"toggle" + (state.bold ? " on" : "")} onClick={() => set("bold", !state.bold)}>
            <div className="sw"/>
            <div style={{display:"flex", flexDirection:"column", gap:"2px"}}>
              <span>Bold layout</span>
              <span className="hint" style={{margin:0}}>
                Floating context card + full-bleed canvas
              </span>
            </div>
          </div>
        </div>

        <div className="hint">
          <span className="k">⌘K</span> command palette ·
          <span className="k" style={{marginLeft:4}}>V L R C A F D E</span> tool keys
        </div>
      </div>
    </div>
  );
};
