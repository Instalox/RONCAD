// Right dock panels — Browser, Inspector, Constraints, Export.
// Content swaps per screen to make each mode feel distinct.

const { Fragment } = React;

function SectionHead({ icon, title, badge, collapsed = false }) {
  return (
    <div className="section-head">
      <span className="ico">{icon}</span>
      <span className="title">{title}</span>
      <span className="spacer" />
      {badge != null && <span className="badge num">{badge}</span>}
      <span className="more"><I.Dots /></span>
    </div>
  );
}

function TreeRow({ depth = 0, chev = null, glyph, label, count, selected, muted }) {
  const cls = ["tree-row"];
  if (depth === 1) cls.push("nested");
  if (depth === 2) cls.push("nested-2");
  if (selected) cls.push("selected");
  if (muted) cls.push("group");
  return (
    <div className={cls.join(" ")}>
      <span className="chev">{chev}</span>
      <span className="glyph">{glyph}</span>
      <span>{label}</span>
      {count != null && <span className="count">{count}</span>}
    </div>
  );
}

function Browser({ screen }) {
  return (
    <>
      <TreeRow chev={<I.ChevronDown size={11}/>} glyph={<I.Origin size={13}/>} label="Origin" muted />
      <TreeRow depth={1} glyph={<I.Plane size={12}/>} label="XY plane" />
      <TreeRow chev={<I.ChevronDown size={11}/>} glyph={<I.Sketch size={13}/>} label="Sketches" muted />
      <TreeRow depth={1} glyph={<I.Sketch size={12}/>} label="Sketch 1"
        count={screen==="sketch" ? "14 ent" : screen==="dimension" ? "9 ent" : screen==="constraint" ? "11 ent" : "11 ent"}
        selected={screen!=="extrude"} />
      <TreeRow chev={<I.ChevronDown size={11}/>} glyph={<I.Box size={13}/>} label="Bodies" muted />
      {screen === "extrude"
        ? <TreeRow depth={1} glyph={<I.Box size={12}/>} label="Body 1" count="1 feat" selected />
        : <TreeRow depth={1} glyph={<I.Box size={12}/>} label="(none yet)" muted /> }
    </>
  );
}

function Inspector({ screen }) {
  if (screen === "sketch") {
    return (
      <>
        <div className="prop-group">
          <div className="g-head">
            Line<span className="tag">l_007</span>
          </div>
          <div className="prop"><span className="lbl">Start</span>
            <span className="val">-20.000<span className="unit">mm</span></span></div>
          <div className="prop"><span className="lbl">End</span>
            <span className="val">34.000<span className="unit">mm</span></span></div>
          <div className="prop"><span className="lbl">Length</span>
            <span className="val editable">58.310<span className="unit">mm</span></span></div>
          <div className="prop"><span className="lbl">Angle</span>
            <span className="val editable">3.18<span className="unit">°</span></span></div>
        </div>
        <div className="help">
          <div className="strong">Line</div>
          Click to set end point. Hold <span className="k">Shift</span> to
          constrain to 15° increments.
        </div>
      </>
    );
  }
  if (screen === "dimension") {
    return (
      <>
        <div className="prop-group">
          <div className="g-head">
            Dimension<span className="tag">d_003</span>
          </div>
          <div className="prop"><span className="lbl">Type</span>
            <span className="val">Linear</span></div>
          <div className="prop"><span className="lbl">Value</span>
            <span className="val editable">50.000<span className="unit">mm</span></span></div>
          <div className="prop"><span className="lbl">Offset</span>
            <span className="val editable">6.000<span className="unit">mm</span></span></div>
          <div className="prop"><span className="lbl">Driving</span>
            <span className="val">Yes</span></div>
        </div>
        <div className="help">
          <div className="strong">Dimension</div>
          Click a second entity to complete. Type a value to override.
        </div>
      </>
    );
  }
  if (screen === "constraint") {
    return (
      <>
        <div className="prop-group">
          <div className="g-head">
            Selection<span className="tag">4 ent</span>
          </div>
          <div className="prop"><span className="lbl">Types</span>
            <span className="val">L×2 · C×2</span></div>
          <div className="prop"><span className="lbl">DOF</span>
            <span className="val">0 remaining</span></div>
          <div className="prop"><span className="lbl">Status</span>
            <span className="val" style={{color:"var(--constraint)"}}>Fully constrained</span></div>
        </div>
        <div className="help">
          Click a constraint tool, then pick entities to apply.
        </div>
      </>
    );
  }
  // extrude
  return (
    <>
      <div className="prop-group">
        <div className="g-head">Extrude<span className="tag">f_001</span></div>
        <div className="prop"><span className="lbl">Profile</span>
          <span className="val">1 selected</span></div>
        <div className="prop"><span className="lbl">Distance</span>
          <span className="val editable">15.000<span className="unit">mm</span></span></div>
        <div className="prop"><span className="lbl">Direction</span>
          <span className="val">+Z</span></div>
        <div className="prop"><span className="lbl">Operation</span>
          <span className="val">New body</span></div>
      </div>
      <div className="help">
        <div className="strong">Extrude</div>
        Drag the handle or type a distance. Press <span className="k">Enter</span> to apply.
      </div>
    </>
  );
}

function Constraints({ screen }) {
  if (screen === "sketch") return <div className="ghost">No constraints yet.</div>;
  if (screen === "dimension") {
    return (
      <>
        <div className="constraint-row"><span className="gl"><I.Dim size={13}/></span>Linear 1<span className="n">d_001</span></div>
        <div className="constraint-row"><span className="gl"><I.Dim size={13}/></span>Linear 2<span className="n">d_002</span></div>
        <div className="constraint-row"><span className="gl"><I.Dim size={13}/></span>Radial 1<span className="n">d_003</span></div>
      </>
    );
  }
  if (screen === "constraint") {
    return (
      <>
        <div className="constraint-row"><span className="gl"><I.Coincident size={13}/></span>Coincident<span className="n">×4</span></div>
        <div className="constraint-row"><span className="gl"><I.Horizontal size={13}/></span>Horizontal<span className="n">×2</span></div>
        <div className="constraint-row"><span className="gl"><I.Vertical size={13}/></span>Vertical<span className="n">×2</span></div>
        <div className="constraint-row"><span className="gl"><I.Parallel size={13}/></span>Parallel<span className="n">×1</span></div>
        <div className="constraint-row"><span className="gl"><I.Equal size={13}/></span>Equal<span className="n">×1</span></div>
      </>
    );
  }
  return (
    <>
      <div className="constraint-row"><span className="gl"><I.Coincident size={13}/></span>Coincident<span className="n">×4</span></div>
      <div className="constraint-row"><span className="gl"><I.Horizontal size={13}/></span>Horizontal<span className="n">×2</span></div>
      <div className="constraint-row"><span className="gl"><I.Equal size={13}/></span>Equal<span className="n">×1</span></div>
    </>
  );
}

function Export({ screen }) {
  return (
    <div style={{padding: "4px"}}>
      <div className="tree-row" style={{color:"var(--text-mid)"}}>
        <span className="glyph"><I.Export size={13}/></span>STL · coming soon
      </div>
      <div className="tree-row" style={{color:"var(--text-mid)"}}>
        <span className="glyph"><I.Export size={13}/></span>STEP · coming soon
      </div>
      <div className="tree-row" style={{color:"var(--text-mid)"}}>
        <span className="glyph"><I.Export size={13}/></span>DXF · coming soon
      </div>
    </div>
  );
}

window.RightDock = function RightDock({ screen }) {
  const badges = {
    sketch:     { browser: "1", cons: 0 },
    dimension:  { browser: "1", cons: 3 },
    constraint: { browser: "1", cons: 10 },
    extrude:    { browser: "1", cons: 3 },
  }[screen];

  return (
    <div className="right-dock">
      <div className="section">
        <SectionHead icon={<I.Rows size={13}/>} title="Browser" badge={badges.browser} />
        <div className="section-body"><Browser screen={screen}/></div>
      </div>
      <div className="section">
        <SectionHead icon={<I.Panel size={13}/>} title="Inspector" />
        <div className="section-body" style={{padding:"6px 4px 10px"}}><Inspector screen={screen}/></div>
      </div>
      <div className="section">
        <SectionHead icon={<I.List size={13}/>} title="Constraints" badge={badges.cons || null} />
        <div className="section-body"><Constraints screen={screen}/></div>
      </div>
      <div className="section">
        <SectionHead icon={<I.Export size={13}/>} title="Export" />
        <div className="section-body"><Export screen={screen}/></div>
      </div>
    </div>
  );
};

// Floating "context card" used in bold mode — replaces the fixed inspector
window.ContextCard = function ContextCard({ screen }) {
  const content = {
    sketch:     { name: "Line · l_007", kind: "DRAWING", rows: [["Length","58.310 mm"],["Angle","3.18°"],["Snap","Endpoint"]] },
    dimension:  { name: "Linear · d_003", kind: "DIMENSION", rows: [["Value","50.000 mm"],["Offset","6.000 mm"],["Driving","Yes"]] },
    constraint: { name: "4 entities", kind: "SELECTION", rows: [["DOF","0 rem"],["Status","Solved"],["Types","L×2 C×2"]] },
    extrude:    { name: "Extrude · f_001", kind: "FEATURE", rows: [["Distance","15.000 mm"],["Dir","+Z"],["Op","New body"]] },
  }[screen];
  return (
    <div className="ctx-card fade-in" style={{ top: 70, right: 20 }}>
      <div className="head">
        <span className="name">{content.name}</span>
        <span className="kind">{content.kind}</span>
      </div>
      <div className="body">
        {content.rows.map(([l, v], i) => (
          <div className="row" key={i}><span className="l">{l}</span><span className="v">{v}</span></div>
        ))}
      </div>
    </div>
  );
};
