// Viewport — renders the drawing canvas for each screen.
// World coordinates in mm, mapped to pixels by a scale factor.

const { useMemo, useState, useRef, useEffect } = React;

function Grid({ style, w, h, ppmm, originX, originY }) {
  if (style === "none") return null;

  // Build grid lines that cover the viewport
  const minor = 5; // mm
  const major = 25; // mm

  const lines = [];
  const dots = [];

  const leftMm  = -originX / ppmm;
  const rightMm = (w - originX) / ppmm;
  const topMm   = -originY / ppmm; // flipped y
  const botMm   = (h - originY) / ppmm;

  if (style === "lines") {
    // Minor vertical
    for (let mm = Math.ceil(leftMm / minor) * minor; mm <= rightMm; mm += minor) {
      const x = originX + mm * ppmm;
      const isMajor = Math.abs(mm % major) < 0.001;
      lines.push(<line key={`v${mm}`} x1={x} y1={0} x2={x} y2={h}
        stroke={isMajor ? "var(--grid-major)" : "var(--grid-minor)"} strokeWidth={1} />);
    }
    for (let mm = Math.ceil(topMm / minor) * minor; mm <= botMm; mm += minor) {
      const y = originY + mm * ppmm;
      const isMajor = Math.abs(mm % major) < 0.001;
      lines.push(<line key={`h${mm}`} x1={0} y1={y} x2={w} y2={y}
        stroke={isMajor ? "var(--grid-major)" : "var(--grid-minor)"} strokeWidth={1} />);
    }
  } else if (style === "dots") {
    for (let mx = Math.ceil(leftMm / minor) * minor; mx <= rightMm; mx += minor) {
      for (let my = Math.ceil(topMm / minor) * minor; my <= botMm; my += minor) {
        const x = originX + mx * ppmm;
        const y = originY + my * ppmm;
        const isMajor = Math.abs(mx % major) < 0.001 && Math.abs(my % major) < 0.001;
        dots.push(<circle key={`${mx},${my}`} cx={x} cy={y} r={isMajor ? 1 : 0.6}
          fill={isMajor ? "var(--grid-major)" : "var(--grid-dot)"} />);
      }
    }
  }

  // Axes
  const axes = (
    <>
      <line x1={0} y1={originY} x2={w} y2={originY} stroke="var(--axis-x)" strokeWidth={1} opacity="0.45" />
      <line x1={originX} y1={0} x2={originX} y2={h} stroke="var(--axis-y)" strokeWidth={1} opacity="0.45" />
    </>
  );

  return (
    <>
      {lines}
      {dots}
      {axes}
    </>
  );
}

// A scene is a set of entities drawn in world mm space.
// Each screen renders a tailored scene.

function SketchScene({ ppmm, ox, oy, selection }) {
  // A bracket: rectangle 60x45 with a hole and a fillet, being actively drawn.
  const w2mm = (xmm) => ox + xmm * ppmm;
  const h2mm = (ymm) => oy - ymm * ppmm; // y up

  const rect = { x: -30, y: -5, w: 60, h: 45 };
  const hole = { cx: 15, cy: 15, r: 6 };
  // incomplete line being drawn from (20, 25) to cursor
  const cursor = { x: 34, y: 28 };

  return (
    <>
      {/* Rectangle */}
      <path
        d={`M ${w2mm(rect.x)} ${h2mm(rect.y)} h ${rect.w*ppmm} v ${-rect.h*ppmm} h ${-rect.w*ppmm} Z`}
        fill="none" stroke="var(--entity-idle)" strokeWidth={1.25}
      />
      {/* Small fillet preview on top-right corner */}
      <path
        d={`M ${w2mm(30-8)} ${h2mm(40)} A ${8*ppmm} ${8*ppmm} 0 0 1 ${w2mm(30)} ${h2mm(40-8)}`}
        fill="none" stroke="var(--accent)" strokeWidth={1.5}
      />
      {/* Hidden corner indicator */}
      <line x1={w2mm(30-8)} y1={h2mm(40)} x2={w2mm(30)} y2={h2mm(40)} stroke="var(--entity-ghost)" strokeWidth={1} strokeDasharray="2 3" />
      <line x1={w2mm(30)} y1={h2mm(40)} x2={w2mm(30)} y2={h2mm(40-8)} stroke="var(--entity-ghost)" strokeWidth={1} strokeDasharray="2 3" />

      {/* Circle (hole) */}
      <circle cx={w2mm(hole.cx)} cy={h2mm(hole.cy)} r={hole.r*ppmm}
        fill="none" stroke="var(--entity-idle)" strokeWidth={1.25} />
      <circle cx={w2mm(hole.cx)} cy={h2mm(hole.cy)} r={1.5} fill="var(--entity-idle)" />

      {/* Line currently being drawn */}
      <line x1={w2mm(-20)} y1={h2mm(25)} x2={w2mm(cursor.x)} y2={h2mm(cursor.y)}
        stroke="var(--accent)" strokeWidth={1.5} />
      <circle cx={w2mm(-20)} cy={h2mm(25)} r={3} fill="var(--bg-deep)" stroke="var(--accent)" strokeWidth={1.5} />

      {/* Snap indicator at cursor */}
      <g>
        <rect x={w2mm(cursor.x)-5} y={h2mm(cursor.y)-5} width="10" height="10"
          fill="none" stroke="var(--snap)" strokeWidth={1.25} />
        <circle cx={w2mm(cursor.x)} cy={h2mm(cursor.y)} r={2} fill="var(--snap)" />
      </g>

      {/* Live length label */}
      <g transform={`translate(${w2mm(cursor.x)+12}, ${h2mm(cursor.y)-14})`}>
        <rect x={0} y={-10} width={76} height={20} rx={3} fill="var(--bg-elev)" stroke="var(--accent-dim)" />
        <text x={8} y={4} fill="var(--accent)" fontFamily="var(--font-mono)" fontSize="11">L 58.31 mm</text>
      </g>

      {/* Cursor */}
      <g transform={`translate(${w2mm(cursor.x)}, ${h2mm(cursor.y)})`}>
        <path d="M0 0 L0 12 L3 9 L5 13 L6.5 12.2 L4.5 8.2 L8.5 8 Z"
          fill="var(--text)" stroke="var(--bg-deep)" strokeWidth={0.6}/>
      </g>
    </>
  );
}

function DimensionScene({ ppmm, ox, oy }) {
  const w2 = (x) => ox + x * ppmm;
  const h2 = (y) => oy - y * ppmm;

  // Rectangle with width and height dimensions, circle with radius
  return (
    <>
      {/* Rect */}
      <path d={`M ${w2(-38)} ${h2(-8)} h ${50*ppmm} v ${-32*ppmm} h ${-50*ppmm} Z`}
        fill="none" stroke="var(--entity-idle)" strokeWidth={1.25}/>
      {/* Circle */}
      <circle cx={w2(28)} cy={h2(8)} r={14*ppmm} fill="none" stroke="var(--entity-idle)" strokeWidth={1.25}/>

      {/* Width dimension (horizontal, top of rect) */}
      <g stroke="var(--accent)" strokeWidth={1}>
        <line x1={w2(-38)} y1={h2(24)} x2={w2(-38)} y2={h2(32)} />
        <line x1={w2(12)}  y1={h2(24)} x2={w2(12)}  y2={h2(32)} />
        <line x1={w2(-38)} y1={h2(30)} x2={w2(12)}  y2={h2(30)} />
        <polygon points={`${w2(-38)},${h2(30)} ${w2(-38)+6},${h2(30)-3} ${w2(-38)+6},${h2(30)+3}`} fill="var(--accent)"/>
        <polygon points={`${w2(12)},${h2(30)} ${w2(12)-6},${h2(30)-3} ${w2(12)-6},${h2(30)+3}`} fill="var(--accent)"/>
      </g>
      <g transform={`translate(${w2(-13)}, ${h2(30)-10})`}>
        <rect x={-22} y={-2} width={44} height={18} rx={2} fill="var(--bg-deep)" stroke="var(--accent-dim)" />
        <text x={0} y={11} textAnchor="middle" fill="var(--accent)" fontFamily="var(--font-mono)" fontSize="12" fontWeight="500">50.00</text>
      </g>

      {/* Height dimension (left of rect) */}
      <g stroke="var(--accent)" strokeWidth={1}>
        <line x1={w2(-46)} y1={h2(-8)} x2={w2(-38)} y2={h2(-8)} />
        <line x1={w2(-46)} y1={h2(24)} x2={w2(-38)} y2={h2(24)} />
        <line x1={w2(-44)} y1={h2(-8)} x2={w2(-44)} y2={h2(24)} />
        <polygon points={`${w2(-44)},${h2(24)} ${w2(-44)-3},${h2(24)-6} ${w2(-44)+3},${h2(24)-6}`} fill="var(--accent)"/>
        <polygon points={`${w2(-44)},${h2(-8)} ${w2(-44)-3},${h2(-8)+6} ${w2(-44)+3},${h2(-8)+6}`} fill="var(--accent)"/>
      </g>
      <g transform={`translate(${w2(-44)-28}, ${h2(8)-10})`}>
        <rect x={-2} y={-2} width={44} height={18} rx={2} fill="var(--bg-deep)" stroke="var(--accent-dim)"/>
        <text x={20} y={11} textAnchor="middle" fill="var(--accent)" fontFamily="var(--font-mono)" fontSize="12" fontWeight="500">32.00</text>
      </g>

      {/* Circle radius dimension */}
      <g stroke="var(--accent)" strokeWidth={1}>
        <line x1={w2(28)} y1={h2(8)} x2={w2(28)+14*ppmm*0.707} y2={h2(8)-14*ppmm*0.707} />
        <line x1={w2(28)+14*ppmm*0.707} y1={h2(8)-14*ppmm*0.707}
              x2={w2(28)+14*ppmm*0.707+24} y2={h2(8)-14*ppmm*0.707-16} />
      </g>
      <g transform={`translate(${w2(28)+14*ppmm*0.707+24}, ${h2(8)-14*ppmm*0.707-26})`}>
        <rect x={0} y={-2} width={52} height={18} rx={2} fill="var(--bg-deep)" stroke="var(--accent-dim)"/>
        <text x={6} y={11} fill="var(--accent)" fontFamily="var(--font-mono)" fontSize="12" fontWeight="500">R 14.00</text>
      </g>

      {/* Endpoint dots */}
      {[[-38,-8],[12,-8],[12,24],[-38,24]].map(([x,y],i)=>(
        <circle key={i} cx={w2(x)} cy={h2(y)} r={2} fill="var(--entity-idle)" />
      ))}
    </>
  );
}

function ConstraintScene({ ppmm, ox, oy }) {
  const w2 = (x) => ox + x * ppmm;
  const h2 = (y) => oy - y * ppmm;

  // A mostly-constrained sketch — bracket with two equal circles, horizontal/vertical, parallel lines
  const pts = {
    a: [-45, -5], b: [25, -5], c: [25, 25], d: [-45, 25],
    c1: [-28, 10], c2: [8, 10],
  };

  return (
    <>
      {/* Selected rect */}
      <path d={`M ${w2(pts.a[0])} ${h2(pts.a[1])} L ${w2(pts.b[0])} ${h2(pts.b[1])} L ${w2(pts.c[0])} ${h2(pts.c[1])} L ${w2(pts.d[0])} ${h2(pts.d[1])} Z`}
        fill="none" stroke="var(--entity-sel)" strokeWidth={1.4}/>

      {/* Two circles (equal constraint) */}
      <circle cx={w2(pts.c1[0])} cy={h2(pts.c1[1])} r={6*ppmm} fill="none" stroke="var(--entity-sel)" strokeWidth={1.4}/>
      <circle cx={w2(pts.c2[0])} cy={h2(pts.c2[1])} r={6*ppmm} fill="none" stroke="var(--entity-sel)" strokeWidth={1.4}/>

      {/* Construction line through circle centers */}
      <line x1={w2(pts.c1[0])-30} y1={h2(pts.c1[1])} x2={w2(pts.c2[0])+30} y2={h2(pts.c2[1])}
        stroke="var(--entity-ghost)" strokeWidth={1} strokeDasharray="3 3" />

      {/* Endpoint dots */}
      {Object.values(pts).map((p,i)=>(
        <circle key={i} cx={w2(p[0])} cy={h2(p[1])} r={2.5}
          fill="var(--bg-deep)" stroke="var(--accent)" strokeWidth={1.2} />
      ))}

      {/* Constraint glyphs */}
      {/* Equal on circles */}
      <g transform={`translate(${w2((pts.c1[0]+pts.c2[0])/2)}, ${h2(pts.c1[1])+0})`}>
        <rect x={-8} y={-8} width={16} height={16} fill="var(--bg-deep)" stroke="var(--constraint)" rx={3}/>
        <path d="M-4 -3 h 8 M-4 3 h 8" stroke="var(--constraint)" strokeWidth={1.2} fill="none"/>
      </g>

      {/* Horizontal glyph on top line */}
      <g transform={`translate(${w2(-10)}, ${h2(25)-16})`}>
        <rect x={-8} y={-8} width={16} height={16} fill="var(--bg-deep)" stroke="var(--constraint)" rx={3}/>
        <text x={0} y={4} textAnchor="middle" fill="var(--constraint)" fontFamily="var(--font-mono)" fontSize="10" fontWeight="600">H</text>
      </g>

      {/* Vertical glyph on right line */}
      <g transform={`translate(${w2(25)+16}, ${h2(10)})`}>
        <rect x={-8} y={-8} width={16} height={16} fill="var(--bg-deep)" stroke="var(--constraint)" rx={3}/>
        <text x={0} y={4} textAnchor="middle" fill="var(--constraint)" fontFamily="var(--font-mono)" fontSize="10" fontWeight="600">V</text>
      </g>

      {/* Coincident origin */}
      <g transform={`translate(${w2(pts.a[0])-14}, ${h2(pts.a[1])+14})`}>
        <rect x={-8} y={-8} width={16} height={16} fill="var(--bg-deep)" stroke="var(--constraint)" rx={3}/>
        <circle cx={0} cy={0} r={4} fill="none" stroke="var(--constraint)" strokeWidth={1}/>
        <circle cx={0} cy={0} r={2} fill="none" stroke="var(--constraint)" strokeWidth={1}/>
      </g>

      {/* Selection bounding box outline */}
      <rect x={w2(pts.a[0])-6} y={h2(pts.c[1])-6}
            width={(pts.b[0]-pts.a[0])*ppmm+12} height={(pts.c[1]-pts.a[1])*ppmm+12}
            fill="none" stroke="var(--accent)" strokeWidth={1} strokeDasharray="4 3" opacity={0.6}/>

      {/* Status marker */}
      <g transform={`translate(${w2(-10)}, ${h2(-20)})`}>
        <rect x={-60} y={-12} width={120} height={22} rx={3} fill="var(--bg-elev)" stroke="var(--sep)"/>
        <circle cx={-48} cy={-1} r={3} fill="var(--constraint)"/>
        <text x={-40} y={3} fill="var(--text)" fontFamily="var(--font-sans)" fontSize="11">Fully constrained</text>
      </g>
    </>
  );
}

function ExtrudeScene({ ppmm, ox, oy }) {
  // 3D iso view — body created from sketch
  const cx = ox, cy = oy;
  const iso = (x, y, z) => [
    cx + (x - y) * ppmm * 0.866,
    cy - (z * ppmm) - (x + y) * ppmm * 0.5,
  ];

  // Box: width=60 (x), depth=40 (y), height=20 (z)
  const W=60, D=40, H=20;
  const p = {
    a: iso(-W/2, -D/2, 0),
    b: iso( W/2, -D/2, 0),
    c: iso( W/2,  D/2, 0),
    d: iso(-W/2,  D/2, 0),
    A: iso(-W/2, -D/2, H),
    B: iso( W/2, -D/2, H),
    C: iso( W/2,  D/2, H),
    D: iso(-W/2,  D/2, H),
  };
  const pt = (q) => `${q[0]},${q[1]}`;

  return (
    <>
      {/* Ground shadow / origin axes */}
      <g>
        {(() => {
          const ox0 = iso(0,0,0);
          const xe  = iso(40,0,0);
          const ye  = iso(0,40,0);
          const ze  = iso(0,0,35);
          return (
            <>
              <line x1={ox0[0]} y1={ox0[1]} x2={xe[0]} y2={xe[1]} stroke="var(--axis-x)" strokeWidth={1.2} opacity={0.75}/>
              <line x1={ox0[0]} y1={ox0[1]} x2={ye[0]} y2={ye[1]} stroke="var(--axis-y)" strokeWidth={1.2} opacity={0.75}/>
              <line x1={ox0[0]} y1={ox0[1]} x2={ze[0]} y2={ze[1]} stroke="#6BB7F3" strokeWidth={1.2} opacity={0.65}/>
              <text x={xe[0]+4} y={xe[1]+10} fill="var(--axis-x)" fontFamily="var(--font-mono)" fontSize="10">X</text>
              <text x={ye[0]-12} y={ye[1]+10} fill="var(--axis-y)" fontFamily="var(--font-mono)" fontSize="10">Y</text>
              <text x={ze[0]-12} y={ze[1]-2} fill="#6BB7F3" fontFamily="var(--font-mono)" fontSize="10">Z</text>
            </>
          );
        })()}
      </g>

      {/* Top face (light) */}
      <polygon points={[p.A,p.B,p.C,p.D].map(pt).join(" ")}
               fill="#2D333C" stroke="var(--text)" strokeWidth={1}/>
      {/* Front face */}
      <polygon points={[p.a,p.b,p.B,p.A].map(pt).join(" ")}
               fill="#22262D" stroke="var(--text)" strokeWidth={1}/>
      {/* Right face (darker) */}
      <polygon points={[p.b,p.c,p.C,p.B].map(pt).join(" ")}
               fill="#1B1E24" stroke="var(--text)" strokeWidth={1}/>

      {/* Hidden edges back */}
      <line x1={p.d[0]} y1={p.d[1]} x2={p.a[0]} y2={p.a[1]} stroke="var(--entity-ghost)" strokeDasharray="3 3"/>
      <line x1={p.d[0]} y1={p.d[1]} x2={p.c[0]} y2={p.c[1]} stroke="var(--entity-ghost)" strokeDasharray="3 3"/>
      <line x1={p.d[0]} y1={p.d[1]} x2={p.D[0]} y2={p.D[1]} stroke="var(--entity-ghost)" strokeDasharray="3 3"/>

      {/* Sketch profile (hollow on top, becomes the extrude) */}
      {(() => {
        const hA = iso(-18, -10, H+0.02);
        const hB = iso( 18, -10, H+0.02);
        const hC = iso( 18,  10, H+0.02);
        const hD = iso(-18,  10, H+0.02);
        return <polygon points={[hA,hB,hC,hD].map(pt).join(" ")}
          fill="none" stroke="var(--accent)" strokeWidth={1.5} strokeDasharray="4 3"/>;
      })()}

      {/* Extrude arrow (preview direction) */}
      {(() => {
        const base = iso(0, 0, H);
        const tip  = iso(0, 0, H+18);
        return (
          <g>
            <line x1={base[0]} y1={base[1]} x2={tip[0]} y2={tip[1]}
              stroke="var(--accent)" strokeWidth={1.5} />
            <polygon points={`${tip[0]-4},${tip[1]+6} ${tip[0]+4},${tip[1]+6} ${tip[0]},${tip[1]-2}`} fill="var(--accent)"/>
            <g transform={`translate(${tip[0]+8}, ${tip[1]-4})`}>
              <rect x={0} y={-10} width={62} height={20} rx={3} fill="var(--bg-elev)" stroke="var(--accent-dim)"/>
              <text x={6} y={4} fill="var(--accent)" fontFamily="var(--font-mono)" fontSize="11">15.00 mm</text>
            </g>
          </g>
        );
      })()}
    </>
  );
}

function ViewNavGizmo() {
  // Small top-right 3D nav widget on extrude screen
  return (
    <svg width="70" height="70" style={{position:"absolute", top: 10, right: 10, opacity: 0.95}}>
      <g transform="translate(35,35)">
        <line x1="0" y1="0" x2="22" y2="8"   stroke="var(--axis-x)" strokeWidth="1.25"/>
        <line x1="0" y1="0" x2="-22" y2="8"  stroke="var(--axis-y)" strokeWidth="1.25"/>
        <line x1="0" y1="0" x2="0" y2="-24" stroke="#6BB7F3" strokeWidth="1.25"/>
        <circle cx="22" cy="8"  r="7" fill="var(--bg-elev)" stroke="var(--axis-x)"/>
        <text x="22" y="12" textAnchor="middle" fill="var(--axis-x)" fontFamily="var(--font-mono)" fontSize="9" fontWeight="600">X</text>
        <circle cx="-22" cy="8" r="7" fill="var(--bg-elev)" stroke="var(--axis-y)"/>
        <text x="-22" y="12" textAnchor="middle" fill="var(--axis-y)" fontFamily="var(--font-mono)" fontSize="9" fontWeight="600">Y</text>
        <circle cx="0" cy="-24" r="7" fill="var(--bg-elev)" stroke="#6BB7F3"/>
        <text x="0" y="-20" textAnchor="middle" fill="#6BB7F3" fontFamily="var(--font-mono)" fontSize="9" fontWeight="600">Z</text>
        <circle cx="0" cy="0" r="3" fill="var(--accent)"/>
      </g>
    </svg>
  );
}

window.Viewport = function Viewport({ screen, gridStyle, bold, cursor }) {
  const ref = useRef(null);
  const [size, setSize] = useState({ w: 900, h: 600 });

  useEffect(() => {
    if (!ref.current) return;
    const measure = () => {
      const r = ref.current.getBoundingClientRect();
      if (r.width > 0 && r.height > 0) {
        setSize({ w: r.width, h: r.height });
      } else {
        // retry next frame until parent lays out
        requestAnimationFrame(measure);
      }
    };
    const ro = new ResizeObserver(measure);
    ro.observe(ref.current);
    measure();
    return () => ro.disconnect();
  }, []);

  const { w, h } = size;
  const ppmm = 5; // pixels per mm (zoom 5×)
  // Origin placement depends on screen: sketch/dim/constraint centered-low; extrude centered
  const ox = screen === "extrude" ? w*0.5 : w*0.48;
  const oy = screen === "extrude" ? h*0.55 : h*0.55;

  const isIso = screen === "extrude";

  return (
    <div ref={ref} className={"viewport " + (isIso ? "iso" : "")}>
      <svg width={w} height={h} style={{display:"block", position:"absolute", top:0, left:0}}>
        <Grid style={isIso ? "none" : gridStyle} w={w} h={h} ppmm={ppmm} originX={ox} originY={oy} />
        {screen === "sketch"     && <SketchScene ppmm={ppmm} ox={ox} oy={oy} />}
        {screen === "dimension"  && <DimensionScene ppmm={ppmm} ox={ox} oy={oy} />}
        {screen === "constraint" && <ConstraintScene ppmm={ppmm} ox={ox} oy={oy} />}
        {screen === "extrude"    && <ExtrudeScene ppmm={ppmm} ox={ox} oy={oy} />}
      </svg>
      {isIso && <ViewNavGizmo />}
    </div>
  );
};
